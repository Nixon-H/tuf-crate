# test-dr.ps1 — validate the DR (hardware breakpoint) hook path end-to-end
# Spawns the 32-bit PowerShell, attaches as debugger, sets DR0=x86 kernel32!DeviceIoControl
# and DR1=x86 ntdll!NtDeviceIoControlFile via the trace4 code, makes the target do UDP
# socket I/O (each op = NtDeviceIoControlFile to \\.\Afd), and checks the CSV gets rows.
$desk = "$env:USERPROFILE\Desktop"
$cs = @'
using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

public static class DRTest {
  [StructLayout(LayoutKind.Sequential)]
  public struct DEBUG_EVENT {
    public uint dwDebugEventCode; public uint dwProcessId; public uint dwThreadId; public uint pad;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 168)] public byte[] u;
  }
  [StructLayout(LayoutKind.Sequential)]
  public struct WOW64_CONTEXT {
    public uint ContextFlags;
    public uint Dr0, Dr1, Dr2, Dr3, Dr6, Dr7;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 112)] public byte[] FloatSave;
    public uint SegGs, SegFs, SegEs, SegDs;
    public uint Edi, Esi, Ebx, Edx, Ecx, Eax, Ebp, Eip, SegCs, EFlags, Esp, SegSs;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 512)] public byte[] ExtendedRegisters;
  }
  [StructLayout(LayoutKind.Sequential)]
  public struct THREADENTRY32 {
    public uint dwSize; public uint cntUsage; public uint th32ThreadID; public uint th32OwnerProcessID;
    public int tpBasePri; public int tpDeltaPri; public uint dwFlags;
  }
  [DllImport("kernel32.dll", SetLastError = true)] public static extern IntPtr OpenProcess(uint a, bool i, int pid);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool CloseHandle(IntPtr h);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool DebugActiveProcess(int pid);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool DebugActiveProcessStop(int pid);
  [DllImport("kernel32.dll")] public static extern bool DebugSetProcessKillOnExit(bool k);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool WaitForDebugEvent(out DEBUG_EVENT ev, uint ms);
  [DllImport("kernel32.dll")] public static extern bool ContinueDebugEvent(int p, int t, uint s);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern IntPtr OpenThread(uint a, bool i, int tid);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool Wow64GetThreadContext(IntPtr t, ref WOW64_CONTEXT c);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool Wow64SetThreadContext(IntPtr t, ref WOW64_CONTEXT c);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern uint SuspendThread(IntPtr t);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern int ResumeThread(IntPtr t);
  [DllImport("kernel32.dll")] public static extern IntPtr CreateToolhelp32Snapshot(uint f, uint pid);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool Thread32First(IntPtr s, ref THREADENTRY32 t);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool Thread32Next(IntPtr s, ref THREADENTRY32 t);
  [DllImport("psapi.dll", SetLastError = true)] public static extern bool EnumProcessModulesEx(IntPtr h, [Out] IntPtr[] m, uint cb, out uint need, uint filter);
  [DllImport("psapi.dll", CharSet = CharSet.Unicode)] public static extern uint GetModuleFileNameEx(IntPtr h, IntPtr m, StringBuilder sb, uint sz);

  public static byte[] Rd(IntPtr h, ulong addr, uint sz) {
    byte[] b = new byte[sz]; IntPtr rd;
    if (!ReadProcessMemoryWrap(h, (IntPtr)(long)addr, b, sz, out rd)) return null;
    if ((long)rd < sz) { byte[] d2 = new byte[(long)rd]; Array.Copy(b, d2, (long)rd); return d2; }
    return b;
  }
  [DllImport("kernel32.dll", SetLastError = true, EntryPoint = "ReadProcessMemory")]
  public static extern bool ReadProcessMemoryWrap(IntPtr h, IntPtr addr, byte[] b, uint sz, out IntPtr rd);

  public static uint U32(byte[] b, int o) { return BitConverter.ToUInt32(b, o); }
  public static ushort U16(byte[] b, int o) { return BitConverter.ToUInt16(b, o); }
  public static string CStr(IntPtr h, ulong a, int max) {
    byte[] b = Rd(h, a, (uint)max); if (b == null) return null;
    int e = Array.IndexOf(b, (byte)0); if (e < 0) e = max;
    return Encoding.ASCII.GetString(b, 0, e);
  }

  public static ulong ResolveExport32(IntPtr h, Dictionary<string, ulong> mods, string func) {
    ulong modBase;
    if (!mods.TryGetValue("ntdll.dll", out modBase)) return 0;
    // generic: resolve in whichever module contains it — caller passes module via mods
    return 0;
  }

  public static int Run(int pid, string csvPath, int seconds) {
    IntPtr h = OpenProcess(0x1FFFFF, false, pid);
    if (h == IntPtr.Zero) { Console.WriteLine("OpenProcess fail err=" + Marshal.GetLastWin32Error()); return 1; }
    if (!DebugActiveProcess(pid)) { Console.WriteLine("DebugActiveProcess fail err=" + Marshal.GetLastWin32Error()); return 1; }
    DebugSetProcessKillOnExit(false);

    // x86 module bases
    Dictionary<string, ulong> mods = new Dictionary<string, ulong>();
    IntPtr[] ml = new IntPtr[2048]; uint need;
    if (EnumProcessModulesEx(h, ml, (uint)(ml.Length * IntPtr.Size), out need, 0x01)) {
      int n = (int)(need / IntPtr.Size); StringBuilder sb = new StringBuilder(1024);
      for (int i = 0; i < n; i++) { sb.Length = 0; GetModuleFileNameEx(h, ml[i], sb, 1024);
        string f = Path.GetFileName(sb.ToString()).ToLowerInvariant(); if (!mods.ContainsKey(f)) mods[f] = (ulong)ml[i]; }
    }
    Console.WriteLine("x86 mods=" + mods.Count + (mods.ContainsKey("ntdll.dll") ? (" ntdll=0x" + mods["ntdll.dll"].ToString("x")) : " NO NTDLL") + (mods.ContainsKey("kernel32.dll") ? (" k32=0x" + mods["kernel32.dll"].ToString("x")) : " NO K32"));

    ulong ntBase = mods.ContainsKey("ntdll.dll") ? mods["ntdll.dll"] : 0;
    ulong k32Base = mods.ContainsKey("kernel32.dll") ? mods["kernel32.dll"] : 0;
    ulong addrNt = 0, addrK32 = 0;
    // inline 32-bit export resolution (offsets 20/24/28/32/36)
    Func<ulong, string, ulong> resolve = null;
    resolve = (baseAddr, func) => {
      byte[] dos = Rd(h, baseAddr, 64); if (dos == null) return 0;
      uint lfa = U32(dos, 0x3C); if (lfa == 0 || lfa > 0x1000) return 0;
      byte[] pe = Rd(h, baseAddr + lfa, 260); if (pe == null) return 0;
      if (U32(pe, 0) != 0x4550 || U16(pe, 4) != 0x14C || U16(pe, 24) != 0x10B) return 0;
      uint expRva = U32(pe, 120), expSz = U32(pe, 124); if (expRva == 0) return 0;
      byte[] ed = Rd(h, baseAddr + expRva, 40); if (ed == null) return 0;
      uint nF = U32(ed, 20), nN = U32(ed, 24), pF = U32(ed, 28), pN = U32(ed, 32), pO = U32(ed, 36);
      if (nN == 0 || nN > 100000) return 0;
      byte[] na = Rd(h, baseAddr + pN, nN * 4); if (na == null) return 0;
      byte[] ords = Rd(h, baseAddr + pO, nN * 2); if (ords == null) return 0;
      byte[] fs = Rd(h, baseAddr + pF, nF * 4); if (fs == null) return 0;
      for (uint i = 0; i < nN; i++) {
        string nm = CStr(h, baseAddr + U32(na, (int)(i * 4)), 80);
        if (nm == func) {
          ushort ord = U16(ords, (int)(i * 2)); uint fn = U32(fs, (int)(ord * 4));
          if (fn >= expRva && fn < expRva + expSz) { string fwd = CStr(h, baseAddr + fn, 80); if (fwd == null) return 0;
            int dot = fwd.IndexOf('.'); if (dot <= 0) return 0; ulong fb;
            if (!mods.TryGetValue((fwd.Substring(0, dot) + ".dll").ToLowerInvariant(), out fb)) return 0;
            return resolve(fb, fwd.Substring(dot + 1)); }
          return baseAddr + fn;
        }
      }
      return 0;
    };
    if (ntBase != 0) addrNt = resolve(ntBase, "NtDeviceIoControlFile");
    if (k32Base != 0) addrK32 = resolve(k32Base, "DeviceIoControl");
    ulong addrSleep = k32Base != 0 ? resolve(k32Base, "Sleep") : 0;
    Console.WriteLine("resolved: NtDeviceIoControlFile=0x" + addrNt.ToString("x") + " DeviceIoControl=0x" + addrK32.ToString("x") + " Sleep=0x" + addrSleep.ToString("x"));
    if (addrNt == 0) { DebugActiveProcessStop(pid); return 1; }

    // consume initial bp
    for (;;) { DEBUG_EVENT ev; if (!WaitForDebugEvent(out ev, 800)) break; ContinueDebugEvent((int)ev.dwProcessId, (int)ev.dwThreadId, 0x00010002); }

    // set DR0=kernel32!DeviceIoControl, DR1=ntdll!NtDeviceIoControlFile on all threads
    IntPtr snap = CreateToolhelp32Snapshot(0x4, 0);
    THREADENTRY32 te = new THREADENTRY32(); te.dwSize = (uint)Marshal.SizeOf(typeof(THREADENTRY32));
    int armed = 0;
    if (Thread32First(snap, ref te)) {
      do {
        if ((int)te.th32OwnerProcessID != pid) continue;
        IntPtr ht = OpenThread(0x1FFFFF, false, (int)te.th32ThreadID);
        if (ht == IntPtr.Zero) continue;
        SuspendThread(ht);
        WOW64_CONTEXT c = new WOW64_CONTEXT(); c.ContextFlags = 0x1001F; c.FloatSave = new byte[112]; c.ExtendedRegisters = new byte[512];
        if (Wow64GetThreadContext(ht, ref c)) {
          c.Dr0 = (uint)addrK32; c.Dr1 = (uint)addrNt; c.Dr2 = (uint)addrSleep; c.Dr7 = 0x7;
          if (Wow64SetThreadContext(ht, ref c)) armed++;
          else Console.WriteLine("DR set fail tid=" + te.th32ThreadID + " err=" + Marshal.GetLastWin32Error());
        }
        ResumeThread(ht); CloseHandle(ht);
      } while (Thread32Next(snap, ref te));
    }
    CloseHandle(snap);
    Console.WriteLine("DR armed on " + armed + " threads");
    // read back DRs from one thread to verify they stuck
    {
      IntPtr snap2 = CreateToolhelp32Snapshot(0x4, 0);
      THREADENTRY32 te2 = new THREADENTRY32(); te2.dwSize = (uint)Marshal.SizeOf(typeof(THREADENTRY32));
      if (Thread32First(snap2, ref te2)) {
        do {
          if ((int)te2.th32OwnerProcessID != pid) continue;
          IntPtr ht = OpenThread(0x1FFFFF, false, (int)te2.th32ThreadID);
          if (ht == IntPtr.Zero) continue;
          SuspendThread(ht);
          WOW64_CONTEXT c = new WOW64_CONTEXT(); c.ContextFlags = 0x1001F; c.FloatSave = new byte[112]; c.ExtendedRegisters = new byte[512];
          if (Wow64GetThreadContext(ht, ref c)) {
            Console.WriteLine("DR readback tid=" + te2.th32ThreadID + " Dr0=0x" + c.Dr0.ToString("x") + " Dr1=0x" + c.Dr1.ToString("x") + " Dr7=0x" + c.Dr7.ToString("x"));
          }
          ResumeThread(ht); CloseHandle(ht);
          break;
        } while (Thread32Next(snap2, ref te2));
      }
      CloseHandle(snap2);
    }

    StreamWriter csv = new StreamWriter(csvPath) { AutoFlush = true };
    csv.WriteLine("ts,tid,handle,ioctl,in_len,out_len,buf_off,status");
    int hits = 0; DateTime deadline = DateTime.Now.AddSeconds(seconds); bool stop = false;
    while (!stop) {
      DEBUG_EVENT ev;
      if (!WaitForDebugEvent(out ev, 500)) { if (DateTime.Now > deadline) stop = true; continue; }
      int p = (int)ev.dwProcessId, t = (int)ev.dwThreadId;
      if (ev.dwDebugEventCode == 5) { Console.WriteLine("TARGET EXITED code=0x" + BitConverter.ToUInt32(ev.u, 0).ToString("x")); break; }
      if (ev.dwDebugEventCode == 1) {
        uint code = BitConverter.ToUInt32(ev.u, 0);
        ulong addr = BitConverter.ToUInt64(ev.u, 16);
        if (hits < 40) Console.WriteLine("EXC code=0x" + code.ToString("x") + " addr=0x" + addr.ToString("x") + " first=" + BitConverter.ToUInt32(ev.u, 152));
        if ((code == 0x80000004 || code == 0x4000001E) && (addr == addrNt || addr == addrK32 || addr == addrSleep)) {
          IntPtr ht = OpenThread(0x1FFFFF, false, t);
          if (ht != IntPtr.Zero) {
            WOW64_CONTEXT c = new WOW64_CONTEXT(); c.ContextFlags = 0x10007; c.FloatSave = new byte[112]; c.ExtendedRegisters = new byte[512];
            if (Wow64GetThreadContext(ht, ref c)) {
              ulong handle, ioctl, inbuf, inlen;
              if (addr == addrK32) {
                handle = BitConverter.ToUInt32(Rd(h, c.Esp + 4, 4) ?? new byte[4], 0);
                byte[] b2 = Rd(h, c.Esp + 8, 4); ioctl = b2 == null ? 0 : BitConverter.ToUInt32(b2, 0);
                byte[] b3 = Rd(h, c.Esp + 0xC, 4); inbuf = b3 == null ? 0 : BitConverter.ToUInt32(b3, 0);
                byte[] b4 = Rd(h, c.Esp + 0x10, 4); inlen = b4 == null ? 0 : BitConverter.ToUInt32(b4, 0);
              } else {
                handle = BitConverter.ToUInt32(Rd(h, c.Esp + 4, 4) ?? new byte[4], 0);
                byte[] b2 = Rd(h, c.Esp + 0x18, 4); ioctl = b2 == null ? 0 : BitConverter.ToUInt32(b2, 0);
                byte[] b3 = Rd(h, c.Esp + 0x1C, 4); inbuf = b3 == null ? 0 : BitConverter.ToUInt32(b3, 0);
                byte[] b4 = Rd(h, c.Esp + 0x20, 4); inlen = b4 == null ? 0 : BitConverter.ToUInt32(b4, 0);
              }
              hits++;
              if (hits <= 5) csv.WriteLine(DateTime.Now.Ticks + "," + t + "," + handle + "," + ioctl + "," + inlen + ",0,0,pre," + p);
            }
            CloseHandle(ht);
          }
        }
        ContinueDebugEvent(p, t, 0x00010002);
        continue;
      }
      ContinueDebugEvent(p, t, 0x00010002);
    }
    csv.Flush(); csv.Close();
    // clear DRs
    snap = CreateToolhelp32Snapshot(0x4, 0);
    te = new THREADENTRY32(); te.dwSize = (uint)Marshal.SizeOf(typeof(THREADENTRY32));
    if (Thread32First(snap, ref te)) {
      do {
        if ((int)te.th32OwnerProcessID != pid) continue;
        IntPtr ht = OpenThread(0x1FFFFF, false, (int)te.th32ThreadID);
        if (ht == IntPtr.Zero) continue;
        SuspendThread(ht);
        WOW64_CONTEXT c = new WOW64_CONTEXT(); c.ContextFlags = 0x1001F; c.FloatSave = new byte[112]; c.ExtendedRegisters = new byte[512];
        if (Wow64GetThreadContext(ht, ref c)) { c.Dr0 = 0; c.Dr1 = 0; c.Dr7 = 0; Wow64SetThreadContext(ht, ref c); }
        ResumeThread(ht); CloseHandle(ht);
      } while (Thread32Next(snap, ref te));
    }
    CloseHandle(snap);
    DebugActiveProcessStop(pid);
    CloseHandle(h);
    Console.WriteLine("TOTAL DR HITS: " + hits);
    return hits > 0 ? 0 : 2;
  }
}
'@
Add-Type -TypeDefinition $cs
Write-Host "=== spawning 32-bit PowerShell with UDP traffic loop ==="
$p = Start-Process "$env:WINDIR\SysWOW64\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList '-NoProfile','-Command','1..60 | ForEach-Object { try { $u = New-Object Net.Sockets.UdpClient; [void]$u.Send([byte[]]@(1,2,3),3,"127.0.0.1",9); $u.Close() } catch {}; Start-Sleep -Milliseconds 1000 }' -WindowStyle Hidden -PassThru
Start-Sleep -Seconds 3
$rc = [DRTest]::Run($p.Id, "$desk\dr-test2.csv", 25)
Write-Host "exit=$rc"
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
