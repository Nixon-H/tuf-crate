# trace2.ps1 — corrected multi-process DeviceIoControl tracer (Phase 2.5, fixed hooks)
# Fixes vs windows-tracer/tuf_trace.c:
#   * DeviceIoControl is exported by kernel32, NOT ntdll (old tracer computed a bogus
#     kernel-space address from GetProcAddress(ntdll,"DeviceIoControl") == NULL and the
#     breakpoint was never armed)
#   * NtDeviceIoControlFile args read from correct x64 slots:
#     6th=ioctl @rsp+0x30, 7th=inbuf @rsp+0x38, 8th=inlen @rsp+0x40
#   * one tracer instance attaches to ALL accessible ASUS processes at once
#   * event loop keeps per-thread single-step state (no livelock, hooks always re-armed)
# Same observation-only policy: int3 on I/O entry, dump input buffer, restore byte,
# single-step over, re-arm. Never constructs or modifies any request.
# Stop: create Desktop\stop.flag (or 12-min deadline). Detach: disarm ALL hooks, drain
# pending single-steps, then DebugActiveProcessStop — targets keep running normally.

$ErrorActionPreference = 'Continue'
$desk = "$env:USERPROFILE\Desktop"
$stat = "$desk\trace2-status.log"
function L2($m) { Add-Content -Path $stat -Value ("[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $m) }
if (Test-Path "$desk\stop.flag") { Remove-Item "$desk\stop.flag" -Force }
Set-Content -Path $stat -Value ""

$cs = @'
using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

public static class Tracer {
  const uint DBG_CONTINUE = 0x00010002;
  const uint DBG_EXCEPTION_NOT_HANDLED = 0x80010001;
  const int EXCEPTION_DEBUG_EVENT = 1;
  const int EXIT_PROCESS_DEBUG_EVENT = 5;
  const uint EXCEPTION_BREAKPOINT = 0x80000003;
  const uint EXCEPTION_SINGLE_STEP = 0x80000004;
  static readonly uint[] FAMILY = { 0x22B80A, 0x22B807, 0x22BB07, 0x22BB0A, 0x22C115, 0x226C15, 0x22B015 };

  [StructLayout(LayoutKind.Sequential)]
  public struct DEBUG_EVENT {
    public uint dwDebugEventCode;
    public uint dwProcessId;
    public uint dwThreadId;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 172)] public byte[] u;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct CONTEXT {
    public ulong P1Home, P2Home, P3Home, P4Home, P5Home, P6Home;
    public uint ContextFlags, MxCsr;
    public ushort SegCs, SegDs, SegEs, SegFs, SegGs, SegSs;
    public uint EFlags;
    public ulong Dr0, Dr1, Dr2, Dr3, Dr6, Dr7;
    public ulong Rax, Rcx, Rdx, Rbx, Rsp, Rbp, Rsi, Rdi, R8, R9, R10, R11, R12, R13, R14, R15, Rip;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 1000)] public byte[] tail;
  }

  [DllImport("kernel32.dll", SetLastError = true)] static extern IntPtr OpenProcess(uint a, bool i, int pid);
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool CloseHandle(IntPtr h);
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool ReadProcessMemory(IntPtr h, IntPtr addr, byte[] b, uint sz, out IntPtr rd);
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool WriteProcessMemory(IntPtr h, IntPtr addr, byte[] b, uint sz, out IntPtr wr);
  [DllImport("kernel32.dll")] static extern bool FlushInstructionCache(IntPtr h, IntPtr addr, uint sz);
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool DebugActiveProcess(int pid);
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool DebugActiveProcessStop(int pid);
  [DllImport("kernel32.dll")] static extern bool DebugSetProcessKillOnExit(bool kill);
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool WaitForDebugEvent(out DEBUG_EVENT ev, uint ms);
  [DllImport("kernel32.dll")] static extern bool ContinueDebugEvent(int pid, int tid, uint st);
  [DllImport("kernel32.dll", SetLastError = true)] static extern IntPtr OpenThread(uint a, bool i, int tid);
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool GetThreadContext(IntPtr t, ref CONTEXT c);
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool SetThreadContext(IntPtr t, ref CONTEXT c);
  [DllImport("kernel32.dll", CharSet = CharSet.Ansi)] static extern IntPtr GetModuleHandle(string n);
  [DllImport("kernel32.dll", CharSet = CharSet.Ansi, SetLastError = true)] static extern IntPtr GetProcAddress(IntPtr m, string n);
  [DllImport("psapi.dll", SetLastError = true)] static extern bool EnumProcessModulesEx(IntPtr h, [Out] IntPtr[] mods, uint cb, out uint needed, uint filter);
  [DllImport("psapi.dll", CharSet = CharSet.Unicode)] static extern uint GetModuleFileNameEx(IntPtr h, IntPtr m, StringBuilder sb, uint sz);
  [DllImport("kernel32.dll")] static extern void GetSystemTimeAsFileTime(out long ft);

  class Target {
    public int pid; public string name; public bool allMode;
    public IntPtr hProc; public StreamWriter csv; public FileStream bins;
    public long bufOff; public bool dead;
  }
  class Hook { public int tidx; public ulong addr; public byte orig; public int kind; public string label; public bool armed; }

  static List<Target> targets = new List<Target>();
  static List<Hook> hooks = new List<Hook>();
  static Dictionary<int, Hook> pendingStep = new Dictionary<int, Hook>(); // tid -> hook awaiting re-arm
  static string desk, statPath;

  static void L(string m) {
    File.AppendAllText(statPath, string.Format("[{0:HH:mm:ss}] {1}\r\n", DateTime.Now, m));
  }

  static ulong TargetModBase(IntPtr hProc, string want) {
    IntPtr[] mods = new IntPtr[2048];
    uint needed;
    if (!EnumProcessModulesEx(hProc, mods, (uint)(mods.Length * IntPtr.Size), out needed, 0x03)) return 0;
    int n = (int)(needed / IntPtr.Size); if (n > mods.Length) n = mods.Length;
    StringBuilder sb = new StringBuilder(1024);
    for (int i = 0; i < n; i++) {
      sb.Length = 0;
      GetModuleFileNameEx(hProc, mods[i], sb, 1024);
      if (Path.GetFileName(sb.ToString()).ToLowerInvariant() == want) return (ulong)mods[i];
    }
    return 0;
  }

  [DllImport("kernel32.dll")] static extern bool IsWow64Process(IntPtr hProc, out bool wow64);
  static bool IsWow64(IntPtr hProc) {
    bool wow;
    if (!IsWow64Process(hProc, out wow)) return false;
    return wow;
  }

  static void ArmHook(Hook hk) {
    Target t = targets[hk.tidx];
    if (t.dead || hk.armed) return;
    IntPtr wr; byte[] cc = new byte[] { 0xCC };
    WriteProcessMemory(t.hProc, (IntPtr)(long)hk.addr, cc, 1, out wr);
    FlushInstructionCache(t.hProc, (IntPtr)(long)hk.addr, 1);
    hk.armed = true;
  }
  static void DisarmHook(Hook hk) {
    Target t = targets[hk.tidx];
    if (t.dead || !hk.armed) return;
    IntPtr wr; byte[] ob = new byte[] { hk.orig };
    WriteProcessMemory(t.hProc, (IntPtr)(long)hk.addr, ob, 1, out wr);
    FlushInstructionCache(t.hProc, (IntPtr)(long)hk.addr, 1);
    hk.armed = false;
  }

  static void Attach(int pid, bool allMode) {
    IntPtr h = OpenProcess(0x1FFFFF, false, pid);
    if (h == IntPtr.Zero) { L(string.Format("ATTACH FAIL OpenProcess pid={0} err={1}", pid, Marshal.GetLastWin32Error())); return; }
    if (IsWow64(h)) { L(string.Format("SKIP WOW64 pid={0}", pid)); CloseHandle(h); return; }
    if (!DebugActiveProcess(pid)) {
      L(string.Format("ATTACH FAIL DebugActiveProcess pid={0} err={1}", pid, Marshal.GetLastWin32Error()));
      CloseHandle(h); return;
    }
    DebugSetProcessKillOnExit(false);
    string name;
    try { name = System.Diagnostics.Process.GetProcessById(pid).ProcessName; }
    catch { name = "pid" + pid; }
    string baseName = (name + "-" + pid).Replace(" ", "_");
    Target t = new Target { pid = pid, name = name, allMode = allMode, hProc = h, bufOff = 0 };
    t.csv = new StreamWriter(new FileStream(Path.Combine(desk, baseName + ".csv"), FileMode.Create, FileAccess.Write)) { AutoFlush = true };
    t.csv.WriteLine("ts,tid,handle,ioctl,in_len,out_len,buf_off,status");
    t.bins = new FileStream(Path.Combine(desk, baseName + ".buffers.bin"), FileMode.Create, FileAccess.Write);
    int tidx = targets.Count; targets.Add(t);

    int armed = 0;
    // kernel32!DeviceIoControl (kernelbase skipped: kernel32 wrapper jumps there, and the
    // ntdll hook records the same logical call; avoids double-int3 livelocks on the jmp path)
    string[][] defs = {
      new string[] { "kernel32.dll", "DeviceIoControl", "kernel32!DeviceIoControl", "0" },
      new string[] { "ntdll.dll", "NtDeviceIoControlFile", "ntdll!NtDeviceIoControlFile", "1" },
      new string[] { "ntdll.dll", "ZwDeviceIoControlFile", "ntdll!ZwDeviceIoControlFile", "1" }
    };
    foreach (var d in defs) {
      IntPtr lm = GetModuleHandle(d[0]);
      IntPtr lp = GetProcAddress(lm, d[1]);
      if (lp == IntPtr.Zero) { L(string.Format("RESOLVE FAIL local {0}!{1}", d[0], d[1])); continue; }
      ulong tgtBase = TargetModBase(h, d[0]);
      if (tgtBase == 0) { L(string.Format("MODULE NOT FOUND pid={0} {1}", pid, d[0])); continue; }
      ulong tgt = tgtBase + ((ulong)lp - (ulong)lm);
      if (tgt >= 0x800000000000) { L(string.Format("INSANE ADDR pid={0} {1:x}", pid, tgt)); continue; }
      byte[] ob = new byte[1]; IntPtr rd;
      if (!ReadProcessMemory(h, (IntPtr)(long)tgt, ob, 1, out rd)) { L(string.Format("ORIG READ FAIL pid={0} {1:x}", pid, tgt)); continue; }
      hooks.Add(new Hook { tidx = tidx, addr = tgt, orig = ob[0], kind = int.Parse(d[3]), label = d[2], armed = false });
      armed++;
      L(string.Format("HOOK pid={0} {1} @ {2:x} orig={3:x2}", pid, d[2], tgt, ob[0]));
    }
    L(string.Format("ATTACHED pid={0} ({1}) mode={2} hooks={3}", pid, name, allMode ? "all" : "family", armed));
  }

  static Hook FindHook(ulong addr) {
    foreach (Hook hk in hooks) if (hk.addr == addr) return hk;
    return null;
  }
  static bool InFamily(ulong ioctl) {
    uint io = (uint)ioctl;
    foreach (uint f in FAMILY) if (f == io) return true;
    return false;
  }
  static ulong RemoteU64(Target t, ulong addr) {
    byte[] b = new byte[8]; IntPtr rd;
    if (!ReadProcessMemory(t.hProc, (IntPtr)(long)addr, b, 8, out rd)) return 0;
    return BitConverter.ToUInt64(b, 0);
  }

  static void LogCall(Target t, Hook hk, CONTEXT ctx, uint tid) {
    ulong handle, ioctl, inbuf, inlen, outlen;
    if (hk.kind == 0) { // DeviceIoControl
      handle = ctx.Rcx; ioctl = ctx.Rdx; inbuf = ctx.R8; inlen = ctx.R9;
      outlen = RemoteU64(t, ctx.Rsp + 0x30);
    } else {            // Nt/ZwDeviceIoControlFile
      handle = ctx.Rcx;
      ioctl = RemoteU64(t, ctx.Rsp + 0x30);
      inbuf = RemoteU64(t, ctx.Rsp + 0x38);
      inlen = RemoteU64(t, ctx.Rsp + 0x40);
      outlen = 0;
    }
    if (!t.allMode && !InFamily(ioctl)) return;
    byte[] data = null;
    if (inbuf != 0 && inlen > 0) {
      uint take = inlen > 512 ? 512u : (uint)inlen;
      byte[] tmp = new byte[take]; IntPtr rd;
      if (ReadProcessMemory(t.hProc, (IntPtr)(long)inbuf, tmp, take, out rd)) {
        if ((long)rd < take) { byte[] d2 = new byte[(long)rd]; Array.Copy(tmp, d2, (long)rd); tmp = d2; }
        data = tmp;
      }
    }
    long ts; GetSystemTimeAsFileTime(out ts);
    long off = 0;
    if (data != null && data.Length > 0) { off = t.bufOff; t.bins.Write(data, 0, data.Length); t.bufOff += data.Length; }
    t.csv.WriteLine(string.Format("{0},{1},{2},{3},{4},{5},{6},pre,{7}", ts, tid, handle, ioctl, inlen, outlen, off, t.pid));
  }

  public static void Run(string[] pidsAll, string outDir, string statFile, string flagFile, int minutes) {
    desk = outDir; statPath = statFile;
    L("=== trace2 (corrected hooks) starting ===");
    foreach (string pa in pidsAll) {
      string[] parts = pa.Split(':');
      int pid; bool all = parts.Length > 1 && parts[1] == "all";
      if (int.TryParse(parts[0], out pid)) Attach(pid, all);
    }
    if (targets.Count == 0) { L("no targets attached - aborting"); return; }

    // consume initial attach breakpoints
    for (;;) {
      DEBUG_EVENT ev;
      if (!WaitForDebugEvent(out ev, 800)) break;
      ContinueDebugEvent((int)ev.dwProcessId, (int)ev.dwThreadId, DBG_CONTINUE);
    }
    foreach (Hook hk in hooks) ArmHook(hk);
    L(string.Format("TRACING {0} targets, {1} hooks - waiting for stop.flag or {2} min", targets.Count, hooks.Count, minutes));

    DateTime deadline = DateTime.Now.AddMinutes(minutes);
    long evCount = 0;
    bool stop = false;
    while (!stop) {
      DEBUG_EVENT ev;
      if (!WaitForDebugEvent(out ev, 500)) {
        if (File.Exists(flagFile) || DateTime.Now > deadline) stop = true;
        continue;
      }
      evCount++;
      if ((evCount & 31) == 0 && File.Exists(flagFile)) { stop = true; }
      int pid = (int)ev.dwProcessId, tid = (int)ev.dwThreadId;
      if (ev.dwDebugEventCode == EXIT_PROCESS_DEBUG_EVENT) {
        int ti = targets.FindIndex(x => x.pid == pid);
        if (ti >= 0) { targets[ti].dead = true; L(string.Format("TARGET EXITED pid={0}", pid)); }
        ContinueDebugEvent(pid, tid, DBG_CONTINUE);
        continue;
      }
      if (ev.dwDebugEventCode == EXCEPTION_DEBUG_EVENT) {
        uint code = BitConverter.ToUInt32(ev.u, 0);
        ulong addr = BitConverter.ToUInt64(ev.u, 16);
        uint first = BitConverter.ToUInt32(ev.u, 152);
        if (code == EXCEPTION_BREAKPOINT && first != 0) {
          Hook hk = FindHook(addr);
          if (hk != null && hk.armed) {
            Target t = targets[hk.tidx];
            IntPtr hThread = OpenThread(0x1FFFFF, false, tid);
            if (hThread != IntPtr.Zero) {
              CONTEXT ctx = new CONTEXT(); ctx.ContextFlags = 0x10000B; ctx.tail = new byte[1000];
              if (GetThreadContext(hThread, ref ctx)) {
                LogCall(t, hk, ctx, (uint)tid);
                DisarmHook(hk);
                ctx.Rip = hk.addr; ctx.EFlags |= 0x100; ctx.ContextFlags = 0x10000B;
                SetThreadContext(hThread, ref ctx);
                pendingStep[tid] = hk;
                ContinueDebugEvent(pid, tid, DBG_CONTINUE);
                CloseHandle(hThread);
                continue;
              }
              CloseHandle(hThread);
            }
            // context failure: leave disarmed, continue safely
            ContinueDebugEvent(pid, tid, DBG_CONTINUE);
            continue;
          }
          ContinueDebugEvent(pid, tid, DBG_CONTINUE); // system or foreign bp
          continue;
        }
        if (code == EXCEPTION_SINGLE_STEP) {
          Hook hk;
          if (pendingStep.TryGetValue(tid, out hk)) { pendingStep.Remove(tid); ArmHook(hk); }
          else {
            // strayed single-step from an interrupted disarm: re-arm matching hook if any
            Hook hk2 = FindHook(addr - 1);
            if (hk2 != null && !hk2.armed) ArmHook(hk2);
          }
          ContinueDebugEvent(pid, tid, DBG_CONTINUE);
          continue;
        }
        if (code == EXCEPTION_BREAKPOINT) { ContinueDebugEvent(pid, tid, DBG_CONTINUE); continue; }
        ContinueDebugEvent(pid, tid, DBG_EXCEPTION_NOT_HANDLED);
        continue;
      }
      ContinueDebugEvent(pid, tid, DBG_CONTINUE);
    }

    // stop: disarm everything, drain pending single-steps, then detach
    L("stopping: disarming all hooks");
    foreach (Hook hk in hooks) DisarmHook(hk);
    DateTime drain = DateTime.Now.AddSeconds(3);
    while (pendingStep.Count > 0 && DateTime.Now < drain) {
      DEBUG_EVENT ev;
      if (WaitForDebugEvent(out ev, 300)) {
        int pid = (int)ev.dwProcessId, tid = (int)ev.dwThreadId;
        if (ev.dwDebugEventCode == EXCEPTION_DEBUG_EVENT) {
          uint code = BitConverter.ToUInt32(ev.u, 0);
          ulong addr = BitConverter.ToUInt64(ev.u, 16);
          if (code == EXCEPTION_SINGLE_STEP) {
            Hook hk;
            if (pendingStep.TryGetValue(tid, out hk)) { pendingStep.Remove(tid); }
            else { Hook hk2 = FindHook(addr - 1); if (hk2 != null) { /* orig already written */ } }
          } else if (code == EXCEPTION_BREAKPOINT) {
            Hook hk = FindHook(addr);
            if (hk != null && pendingStep.ContainsKey(tid) == false) {
              // rare late bp on disarmed hook: it is disarmed, so continue normally
            }
          }
        }
        ContinueDebugEvent(pid, tid, DBG_CONTINUE);
      }
    }
    foreach (Target t in targets) {
      if (t.dead) continue;
      DebugActiveProcessStop(t.pid);
      t.csv.Flush(); t.csv.Close(); t.bins.Flush(); t.bins.Close();
      CloseHandle(t.hProc);
      L(string.Format("DETACHED pid={0}", t.pid));
    }
    L("=== trace2 complete ===");
  }
}
'@

try { Add-Type -TypeDefinition $cs -Language CSharp -ErrorAction Stop }
catch {
  L2 ("ADD-TYPE ERROR: " + $_.Exception.Message)
  if ($_.Exception.InnerException) { L2 ("DETAIL: " + $_.Exception.InnerException.Message.Substring(0, [Math]::Min(2000, $_.Exception.InnerException.Message.Length))) }
  exit 1
}
if (-not ("Tracer" -as [type])) { L2 "Add-Type FAILED - tracer type missing"; exit 1 }

# ---- resolve ASUS targets fresh ----
$patterns = @(
  @{ n = 'asus_framework'; all = 1 }, @{ n = 'AsusOptimization'; all = 1 },
  @{ n = 'ArmouryCrateControlInterface'; all = 1 }, @{ n = 'ArmouryCrate\.Service'; all = 1 },
  @{ n = 'ArmourySocketServer'; all = 1 }, @{ n = 'ArmouryHtmlDebugServer'; all = 1 },
  @{ n = 'ArmouryCrate$'; all = 1 }, @{ n = 'UserSessionHelper'; all = 1 },
  @{ n = 'AsusAppService'; all = 0 }, @{ n = 'AsusSwitch'; all = 0 },
  @{ n = 'AsusSystemAnalysis'; all = 0 }, @{ n = 'AsusSystemDiagnosis'; all = 0 },
  @{ n = 'AsusOptimizationStartup'; all = 0 }, @{ n = 'AsusCertService'; all = 0 },
  @{ n = 'AuraWallpaper'; all = 0 }, @{ n = 'GameSDK'; all = 0 }, @{ n = 'ROGLiveService'; all = 0 }
)
$seen = @{}
$argList = @()
foreach ($p in $patterns) {
  Get-Process | Where-Object { $_.ProcessName -match $p.n -and -not $seen.ContainsKey($_.Id) } | ForEach-Object {
    $seen[$_.Id] = $true
    $mode = if ($p.all -eq 1) { 'all' } else { 'fam' }
    $argList += ("{0}:{1}" -f $_.Id, $mode)
    L2 ("QUEUED {0} pid={1} mode={2}" -f $_.ProcessName, $_.Id, $mode)
  }
}

[Tracer]::Run($argList, $desk, $stat, "$desk\stop.flag", 12)
L2 "=== trace2.ps1 script end ==="
