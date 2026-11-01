# trace3.ps1 — corrected tracer + WOW64 (32-bit) target support — Phase 2.5
# History: tuf_trace.c hooked a bogus ntdll!DeviceIoControl address (that export does not
# exist in ntdll) and misread Nt stack args; trace2 fixed x64 hooks but skipped
# asus_framework.exe because it is a 32-bit (WOW64) process — and it is the statically
# confirmed AsIO3 client. trace3 adds:
#   * 32-bit module enumeration (EnumProcessModulesEx LIST_MODULES_32BIT from x64)
#   * in-memory 32-bit PE export-table parsing (no cross-arch GetProcAddress possible)
#   * WOW64_CONTEXT get/set + x86 stdcall stack-arg reads
#   * forwarder-export resolution (kernel32!DeviceIoControl forwards to kernelbase on x86)
#   * skips the Armoury components that anti-debug self-exit on attach (leave them alone)
# Observation-only: int3 on I/O entry, dump input buffer, restore byte, single-step over,
# re-arm. Never constructs or modifies any request.
# Stop: create Desktop\stop.flag (or 12-min deadline). Detach: disarm all, drain steps,
# DebugActiveProcessStop — targets keep running normally.

$ErrorActionPreference = 'Continue'
$desk = "$env:USERPROFILE\Desktop"
$stat = "$desk\trace3-status.log"
function L2($m) { Add-Content -Path $stat -Value ("[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $m) }
if (Test-Path "$desk\stop.flag") { Remove-Item "$desk\stop.flag" -Force }
Set-Content -Path $stat -Value ""

$cs = @'
using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

public static class Tracer3 {
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

  // x86 (WOW64) CONTEXT: ContextFlags, Dr0..Dr7, FloatSave(112), SegGs..SegSs,
  // Edi..SegSs, ExtendedRegisters(512) — total 716
  [StructLayout(LayoutKind.Sequential)]
  public struct WOW64_CONTEXT {
    public uint ContextFlags;
    public uint Dr0, Dr1, Dr2, Dr3, Dr6, Dr7;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 112)] public byte[] FloatSave;
    public uint SegGs, SegFs, SegEs, SegDs;
    public uint Edi, Esi, Ebx, Edx, Ecx, Eax, Ebp, Eip, SegCs, EFlags, Esp, SegSs;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 512)] public byte[] ExtendedRegisters;
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
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool Wow64GetThreadContext(IntPtr t, ref WOW64_CONTEXT c);
  [DllImport("kernel32.dll", SetLastError = true)] static extern bool Wow64SetThreadContext(IntPtr t, ref WOW64_CONTEXT c);
  [DllImport("kernel32.dll")] static extern bool IsWow64Process(IntPtr hProc, out bool wow64);
  [DllImport("kernel32.dll", CharSet = CharSet.Ansi)] static extern IntPtr GetModuleHandle(string n);
  [DllImport("kernel32.dll", CharSet = CharSet.Ansi, SetLastError = true)] static extern IntPtr GetProcAddress(IntPtr m, string n);
  [DllImport("psapi.dll", SetLastError = true)] static extern bool EnumProcessModulesEx(IntPtr h, [Out] IntPtr[] mods, uint cb, out uint needed, uint filter);
  [DllImport("psapi.dll", CharSet = CharSet.Unicode)] static extern uint GetModuleFileNameEx(IntPtr h, IntPtr m, StringBuilder sb, uint sz);
  [DllImport("kernel32.dll")] static extern void GetSystemTimeAsFileTime(out long ft);

  class Target {
    public int pid; public string name; public bool allMode; public bool wow64;
    public IntPtr hProc; public StreamWriter csv; public FileStream bins;
    public long bufOff; public bool dead;
    public Dictionary<string, ulong> modBases = new Dictionary<string, ulong>(); // lower name -> base
  }
  class Hook { public int tidx; public ulong addr; public byte orig; public int kind; public string label; public bool armed; }
  // kind 0: x64 kernel32!DeviceIoControl (rcx/rdx/r8/r9)
  // kind 1: x64 ntdll Nt/ZwDeviceIoControlFile ([rsp+30/38/40])
  // kind 2: x86 kernel32!DeviceIoControl (stdcall [esp+8/0xC/0x10])
  // kind 3: x86 ntdll Nt/ZwDeviceIoControlFile ([esp+0x18/0x1C/0x20])

  static List<Target> targets = new List<Target>();
  static List<Hook> hooks = new List<Hook>();
  static Dictionary<int, Hook> pendingStep = new Dictionary<int, Hook>();
  static string desk, statPath;

  static void L(string m) {
    File.AppendAllText(statPath, string.Format("[{0:HH:mm:ss}] {1}\r\n", DateTime.Now, m));
  }

  static void CacheMods(Target t, uint filter) {
    IntPtr[] mods = new IntPtr[2048];
    uint needed;
    if (!EnumProcessModulesEx(t.hProc, mods, (uint)(mods.Length * IntPtr.Size), out needed, filter)) return;
    int n = (int)(needed / IntPtr.Size); if (n > mods.Length) n = mods.Length;
    StringBuilder sb = new StringBuilder(1024);
    for (int i = 0; i < n; i++) {
      sb.Length = 0;
      GetModuleFileNameEx(t.hProc, mods[i], sb, 1024);
      string f = Path.GetFileName(sb.ToString()).ToLowerInvariant();
      if (!t.modBases.ContainsKey(f)) t.modBases[f] = (ulong)mods[i];
    }
  }

  // ---- 32-bit in-memory PE export resolution ----
  static byte[] Rd(Target t, ulong addr, uint sz) {
    byte[] b = new byte[sz]; IntPtr rd;
    if (!ReadProcessMemory(t.hProc, (IntPtr)(long)addr, b, sz, out rd)) return null;
    if ((long)rd < sz) { byte[] d2 = new byte[(long)rd]; Array.Copy(b, d2, (long)rd); return d2; }
    return b;
  }
  static uint U32(byte[] b, int o) { return BitConverter.ToUInt32(b, o); }
  static ushort U16(byte[] b, int o) { return BitConverter.ToUInt16(b, o); }

  static string ReadCStr(Target t, ulong addr, int max) {
    byte[] b = Rd(t, addr, (uint)max);
    if (b == null) return null;
    int e = Array.IndexOf(b, (byte)0);
    if (e < 0) e = max;
    return Encoding.ASCII.GetString(b, 0, e);
  }

  // resolve export 'func' of the module at 'modBase' in target memory; follows forwarders once
  static ulong ResolveExport32(Target t, ulong modBase, string func, int depth) {
    if (depth > 2 || modBase == 0) return 0;
    byte[] dos = Rd(t, modBase, 64); if (dos == null) return 0;
    uint lfa = U32(dos, 0x3C); if (lfa == 0 || lfa > 0x1000) return 0;
    byte[] pe = Rd(t, modBase + lfa, 248); if (pe == null) return 0;
    if (U32(pe, 0) != 0x00004550) return 0;             // "PE\0\0"
    ushort machine = U16(pe, 4);
    if (machine != 0x014C) { L(string.Format("PE32 machine mismatch {0:x}", machine)); return 0; }
    ushort magic = U16(pe, 24);                          // optional header magic (pe[] starts at lfa)
    if (magic != 0x10B) return 0;                        // PE32 only
    uint expDirRva = U32(pe, 24 + 96);                   // data directory[0].VirtualAddress
    uint expDirSize = U32(pe, 24 + 96 + 4);
    if (expDirRva == 0 || expDirSize == 0) return 0;
    byte[] ed = Rd(t, modBase + expDirRva, 40); if (ed == null) return 0;
    uint nFuncs = U32(ed, 20), nNames = U32(ed, 24), pFunctions = U32(ed, 28), pNames = U32(ed, 32), pOrds = U32(ed, 36);
    if (nNames == 0 || nNames > 100000) return 0;
    byte[] names = Rd(t, modBase + pNames, nNames * 4); if (names == null) return 0;
    byte[] ords = Rd(t, modBase + pOrds, nNames * 2); if (ords == null) return 0;
    byte[] funcs = Rd(t, modBase + pFunctions, nFuncs * 4); if (funcs == null) return 0;
    for (uint i = 0; i < nNames; i++) {
      uint nameRva = U32(names, (int)(i * 4));
      string nm = ReadCStr(t, modBase + nameRva, 80);
      if (nm == func) {
        ushort ord = U16(ords, (int)(i * 2));
        uint fnRva = U32(funcs, (int)(ord * 4));
        if (fnRva >= expDirRva && fnRva < expDirRva + expDirSize) {
          // forwarder: "module.function"
          string fwd = ReadCStr(t, modBase + fnRva, 80);
          if (fwd == null) return 0;
          int dot = fwd.IndexOf('.');
          if (dot <= 0) return 0;
          string fmod = (fwd.Substring(0, dot) + ".dll").ToLowerInvariant();
          string ffn = fwd.Substring(dot + 1);
          ulong fbase;
          if (!t.modBases.TryGetValue(fmod, out fbase)) return 0;
          return ResolveExport32(t, fbase, ffn, depth + 1);
        }
        return modBase + fnRva;
      }
    }
    return 0;
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

  static void AddHook(Target t, int tidx, ulong addr, int kind, string label) {
    foreach (Hook e in hooks) if (e.tidx == tidx && e.addr == addr) return; // Nt/Zw alias dedupe
    byte[] ob = new byte[1]; IntPtr rd;
    if (!ReadProcessMemory(t.hProc, (IntPtr)(long)addr, ob, 1, out rd)) { L(string.Format("ORIG READ FAIL pid={0} {1:x}", t.pid, addr)); return; }
    hooks.Add(new Hook { tidx = tidx, addr = addr, orig = ob[0], kind = kind, label = label, armed = false });
    L(string.Format("HOOK pid={0} {1} @ {2:x} orig={3:x2}", t.pid, label, addr, ob[0]));
  }

  static void Attach(int pid, bool allMode) {
    IntPtr h = OpenProcess(0x1FFFFF, false, pid);
    if (h == IntPtr.Zero) { L(string.Format("ATTACH FAIL OpenProcess pid={0} err={1}", pid, Marshal.GetLastWin32Error())); return; }
    bool wow; IsWow64Process(h, out wow);
    if (!DebugActiveProcess(pid)) {
      L(string.Format("ATTACH FAIL DebugActiveProcess pid={0} err={1}", pid, Marshal.GetLastWin32Error()));
      CloseHandle(h); return;
    }
    DebugSetProcessKillOnExit(false);
    string name;
    try { name = System.Diagnostics.Process.GetProcessById(pid).ProcessName; }
    catch { name = "pid" + pid; }
    string baseName = (name + "-" + pid).Replace(" ", "_");
    Target t = new Target { pid = pid, name = name, allMode = allMode, wow64 = wow, hProc = h, bufOff = 0 };
    t.csv = new StreamWriter(new FileStream(Path.Combine(desk, baseName + ".csv"), FileMode.Create, FileAccess.Write)) { AutoFlush = true };
    t.csv.WriteLine("ts,tid,handle,ioctl,in_len,out_len,buf_off,status");
    t.bins = new FileStream(Path.Combine(desk, baseName + ".buffers.bin"), FileMode.Create, FileAccess.Write);
    int tidx = targets.Count; targets.Add(t);

    int armed = 0;
    if (wow) {
      CacheMods(t, 0x01); // LIST_MODULES_32BIT (x64 caller, WOW64 target)
      ulong k32, nt;
      if (!t.modBases.TryGetValue("kernel32.dll", out k32)) { L(string.Format("x86 kernel32 not found pid={0}", pid)); }
      if (!t.modBases.TryGetValue("ntdll.dll", out nt)) { L(string.Format("x86 ntdll not found pid={0}", pid)); }
      ulong a;
      if (k32 != 0) { a = ResolveExport32(t, k32, "DeviceIoControl", 0); if (a != 0) { AddHook(t, tidx, a, 2, "x86 kernel32!DeviceIoControl"); armed++; } }
      if (nt != 0) {
        a = ResolveExport32(t, nt, "NtDeviceIoControlFile", 0); if (a != 0) { AddHook(t, tidx, a, 3, "x86 ntdll!NtDeviceIoControlFile"); armed++; }
        a = ResolveExport32(t, nt, "ZwDeviceIoControlFile", 0); if (a != 0) { AddHook(t, tidx, a, 3, "x86 ntdll!ZwDeviceIoControlFile"); armed++; }
      }
    } else {
      CacheMods(t, 0x03); // LIST_MODULES_ALL (x64)
      ulong k32, nt;
      t.modBases.TryGetValue("kernel32.dll", out k32);
      t.modBases.TryGetValue("ntdll.dll", out nt);
      IntPtr lm = GetModuleHandle("kernel32.dll");
      IntPtr lp = GetProcAddress(lm, "DeviceIoControl");
      if (k32 != 0 && lp != IntPtr.Zero) { AddHook(t, tidx, k32 + ((ulong)lp - (ulong)lm), 0, "kernel32!DeviceIoControl"); armed++; }
      lm = GetModuleHandle("ntdll.dll");
      lp = GetProcAddress(lm, "NtDeviceIoControlFile");
      if (nt != 0 && lp != IntPtr.Zero) { AddHook(t, tidx, nt + ((ulong)lp - (ulong)lm), 1, "ntdll!NtDeviceIoControlFile"); armed++; }
      lp = GetProcAddress(lm, "ZwDeviceIoControlFile");
      if (nt != 0 && lp != IntPtr.Zero) { AddHook(t, tidx, nt + ((ulong)lp - (ulong)lm), 1, "ntdll!ZwDeviceIoControlFile"); armed++; }
    }
    L(string.Format("ATTACHED pid={0} ({1}) mode={2} wow64={3} hooks={4}", pid, name, allMode ? "all" : "family", wow, armed));
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
    byte[] b = Rd(t, addr, 8);
    if (b == null || b.Length < 8) return 0;
    return BitConverter.ToUInt64(b, 0);
  }
  static ulong RemoteU32(Target t, ulong addr) {
    byte[] b = Rd(t, addr, 4);
    if (b == null || b.Length < 4) return 0;
    return BitConverter.ToUInt32(b, 0);
  }

  static void LogCall(Target t, Hook hk, ulong handle, ulong ioctl, ulong inbuf, ulong inlen, ulong outlen, uint tid) {
    if (!t.allMode && !InFamily(ioctl)) return;
    byte[] data = null;
    if (inbuf != 0 && inlen > 0) {
      uint take = inlen > 512 ? 512u : (uint)inlen;
      byte[] tmp = Rd(t, inbuf, take);
      if (tmp != null && tmp.Length > 0) data = tmp;
    }
    long ts; GetSystemTimeAsFileTime(out ts);
    long off = 0;
    if (data != null && data.Length > 0) { off = t.bufOff; t.bins.Write(data, 0, data.Length); t.bufOff += data.Length; }
    t.csv.WriteLine(string.Format("{0},{1},{2},{3},{4},{5},{6},pre,{7}", ts, tid, handle, ioctl, inlen, outlen, off, t.pid));
  }

  public static void Run(string[] pidsAll, string outDir, string statFile, string flagFile, int minutes) {
    desk = outDir; statPath = statFile;
    L("=== trace3 (x64 + WOW64) starting ===");
    foreach (string pa in pidsAll) {
      string[] parts = pa.Split(':');
      int pid; bool all = parts.Length > 1 && parts[1] == "all";
      if (int.TryParse(parts[0], out pid)) Attach(pid, all);
    }
    if (targets.Count == 0) { L("no targets attached - aborting"); return; }

    for (;;) { DEBUG_EVENT ev; if (!WaitForDebugEvent(out ev, 800)) break; ContinueDebugEvent((int)ev.dwProcessId, (int)ev.dwThreadId, DBG_CONTINUE); }
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
      if ((evCount & 31) == 0 && File.Exists(flagFile)) stop = true;
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
            bool handled = false;
            if (hThread != IntPtr.Zero) {
              if (!t.wow64) {
                CONTEXT ctx = new CONTEXT(); ctx.ContextFlags = 0x10000B; ctx.tail = new byte[1000];
                if (GetThreadContext(hThread, ref ctx)) {
                  ulong handle, ioctl, inbuf, inlen, outlen;
                  if (hk.kind == 0) {
                    handle = ctx.Rcx; ioctl = ctx.Rdx; inbuf = ctx.R8; inlen = ctx.R9;
                    outlen = RemoteU64(t, ctx.Rsp + 0x30);
                  } else {
                    handle = ctx.Rcx;
                    ioctl = RemoteU64(t, ctx.Rsp + 0x30);
                    inbuf = RemoteU64(t, ctx.Rsp + 0x38);
                    inlen = RemoteU64(t, ctx.Rsp + 0x40);
                    outlen = 0;
                  }
                  LogCall(t, hk, handle, ioctl, inbuf, inlen, outlen, (uint)tid);
                  DisarmHook(hk);
                  ctx.Rip = hk.addr; ctx.EFlags |= 0x100; ctx.ContextFlags = 0x10000B;
                  SetThreadContext(hThread, ref ctx);
                  pendingStep[tid] = hk;
                  handled = true;
                }
              } else {
                WOW64_CONTEXT c32 = new WOW64_CONTEXT(); c32.ContextFlags = 0x10007; c32.FloatSave = new byte[112]; c32.ExtendedRegisters = new byte[512];
                if (Wow64GetThreadContext(hThread, ref c32)) {
                  ulong handle, ioctl, inbuf, inlen;
                  if (hk.kind == 2) {
                    handle = RemoteU32(t, c32.Esp + 4);
                    ioctl = RemoteU32(t, c32.Esp + 8);
                    inbuf = RemoteU32(t, c32.Esp + 0xC);
                    inlen = RemoteU32(t, c32.Esp + 0x10);
                  } else {
                    handle = RemoteU32(t, c32.Esp + 4);
                    ioctl = RemoteU32(t, c32.Esp + 0x18);
                    inbuf = RemoteU32(t, c32.Esp + 0x1C);
                    inlen = RemoteU32(t, c32.Esp + 0x20);
                  }
                  LogCall(t, hk, handle, ioctl, inbuf, inlen, 0, (uint)tid);
                  DisarmHook(hk);
                  c32.Eip = (uint)hk.addr; c32.EFlags |= 0x100; c32.ContextFlags = 0x10007;
                  Wow64SetThreadContext(hThread, ref c32);
                  pendingStep[tid] = hk;
                  handled = true;
                }
              }
              CloseHandle(hThread);
            }
            ContinueDebugEvent(pid, tid, DBG_CONTINUE);
            if (!handled) L(string.Format("HIT NOT HANDLED pid={0} hook={1}", pid, hk.label));
            continue;
          }
          ContinueDebugEvent(pid, tid, DBG_CONTINUE);
          continue;
        }
        if (code == EXCEPTION_SINGLE_STEP) {
          Hook hk;
          if (pendingStep.TryGetValue(tid, out hk)) { pendingStep.Remove(tid); ArmHook(hk); }
          ContinueDebugEvent(pid, tid, DBG_CONTINUE);
          continue;
        }
        if (code == EXCEPTION_BREAKPOINT) { ContinueDebugEvent(pid, tid, DBG_CONTINUE); continue; }
        ContinueDebugEvent(pid, tid, DBG_EXCEPTION_NOT_HANDLED);
        continue;
      }
      ContinueDebugEvent(pid, tid, DBG_CONTINUE);
    }

    L("stopping: disarming all hooks");
    foreach (Hook hk in hooks) DisarmHook(hk);
    DateTime drain = DateTime.Now.AddSeconds(3);
    while (pendingStep.Count > 0 && DateTime.Now < drain) {
      DEBUG_EVENT ev;
      if (WaitForDebugEvent(out ev, 300)) {
        ContinueDebugEvent((int)ev.dwProcessId, (int)ev.dwThreadId, DBG_CONTINUE);
        if (ev.dwDebugEventCode == EXCEPTION_DEBUG_EVENT &&
            BitConverter.ToUInt32(ev.u, 0) == EXCEPTION_SINGLE_STEP) {
          Hook hk;
          if (pendingStep.TryGetValue((int)ev.dwThreadId, out hk)) pendingStep.Remove((int)ev.dwThreadId);
        }
      }
    }
    foreach (Target t in targets) {
      if (t.dead) continue;
      DebugActiveProcessStop(t.pid);
      t.csv.Flush(); t.csv.Close(); t.bins.Flush(); t.bins.Close();
      CloseHandle(t.hProc);
      L(string.Format("DETACHED pid={0}", t.pid));
    }
    L("=== trace3 complete ===");
  }
}
'@

try { Add-Type -TypeDefinition $cs -Language CSharp -ErrorAction Stop }
catch {
  L2 ("ADD-TYPE ERROR: " + $_.Exception.Message)
  if ($_.Exception.InnerException) { L2 ("DETAIL: " + $_.Exception.InnerException.Message.Substring(0, [Math]::Min(2000, $_.Exception.InnerException.Message.Length))) }
  exit 1
}
if (-not ("Tracer3" -as [type])) { L2 "Add-Type FAILED - tracer type missing"; exit 1 }

# ---- resolve targets fresh ----
# skipped on purpose (anti-debug self-exit on attach, observed in trace2):
#   ArmouryCrate, ArmouryCrate.Service, ArmourySocketServer, ArmouryHtmlDebugServer,
#   UserSessionHelper, AuraWallpaperService, ROGLiveService
$targets = @(
  @{ n = 'asus_framework'; all = 1 }, @{ n = 'AsusOptimization'; all = 1 },
  @{ n = 'AsusOptimizationStartup'; all = 1 }, @{ n = 'ArmouryCrateControlInterface'; all = 1 },
  @{ n = 'AsusAppService'; all = 0 }, @{ n = 'AsusSwitch'; all = 0 },
  @{ n = 'AsusSystemAnalysis'; all = 0 }, @{ n = 'AsusSystemDiagnosis'; all = 0 },
  @{ n = 'AsusCertService'; all = 0 }, @{ n = 'GameSDK'; all = 0 }
)
$seen = @{}
$argList = @()
foreach ($p in $targets) {
  Get-Process | Where-Object { $_.ProcessName -match $p.n -and -not $seen.ContainsKey($_.Id) } | ForEach-Object {
    $seen[$_.Id] = $true
    $mode = if ($p.all -eq 1) { 'all' } else { 'fam' }
    $argList += ("{0}:{1}" -f $_.Id, $mode)
    L2 ("QUEUED {0} pid={1} mode={2}" -f $_.ProcessName, $_.Id, $mode)
  }
}

[Tracer3]::Run($argList, $desk, $stat, "$desk\stop.flag", 12)
L2 "=== trace3.ps1 script end ==="
