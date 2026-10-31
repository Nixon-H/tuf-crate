/*
 * tuf_trace.c — supervised DeviceIoControl tracer for Armoury Crate RGB research.
 *
 * Full source is published in the tuf-crate repository for review before use.
 * What it does: attaches as a debugger to a target process (e.g.
 * asus_framework.exe), sets int3 breakpoints on ntdll!DeviceIoControl and
 * ntdll!NtDeviceIoControlFile, and for every hit logs the IOCTL, buffer
 * lengths and input-buffer contents to a CSV. It ONLY observes — it never
 * constructs, modifies or replays any request; breakpoints are restored and
 * execution continues normally.
 *
 * Usage (elevated cmd):  tuf_trace.exe <PID> <output-base> [all]
 *   <PID>          decimal process id of the process to trace
 *   <output-base>  base path for "base.csv" and "base.buffers.bin"
 *   all            optional: log every IOCTL, not only the AsIO family
 *
 * Build (cross):  x86_64-w64-mingw32-gcc -O2 -o tuf_trace.exe tuf_trace.c
 */

#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <tlhelp32.h>
#include <psapi.h>

#pragma pack(push)
#pragma pack(1)

#pragma pack(push, 1)
typedef struct _TRACE_REC {
    unsigned long long ts;        /* GetSystemTimeAsFileTime */
    unsigned long long tid;
    unsigned long long handle;
    unsigned long long ioctl;
    unsigned long long in_len;
    unsigned long long out_len;
    unsigned long long buf_off;   /* offset into buffers file, 0 = none */
    unsigned long long status;    /* NTSTATUS captured at entry (pre-call) */
    unsigned int       pid;
} TRACE_REC;
#pragma pack(pop)

static HANDLE g_hProcess = NULL;
static char   g_outbase[MAX_PATH];
static FILE  *g_csv = NULL;
static FILE  *g_buf = NULL;
static unsigned long long g_buf_off = 0;
static int    g_log_all = 0;

/* hooked functions: address + original byte */
static struct { unsigned long long addr; unsigned char orig; const char *name; } hooks[2];
static int hook_count = 0;

static void put_breakpoint(unsigned long long addr, const char *name)
{
    unsigned char orig = 0;
    SIZE_T n = 0;
    ReadProcessMemory(g_hProcess, (LPCVOID)(uintptr_t)addr, &orig, 1, &n);
    unsigned char cc = 0xCC;
    WriteProcessMemory(g_hProcess, (LPVOID)(uintptr_t)addr, &cc, 1, &n);
    FlushInstructionCache(g_hProcess, (LPCVOID)(uintptr_t)addr, 1);
    hooks[hook_count].addr = addr;
    hooks[hook_count].orig = orig;
    hooks[hook_count].name = name;
    hook_count++;
}

static void arm_all(void)
{
    for (int i = 0; i < hook_count; i++) {
        unsigned char cc = 0xCC;
        SIZE_T n = 0;
        WriteProcessMemory(g_hProcess, (LPVOID)(uintptr_t)hooks[i].addr, &cc, 1, &n);
        FlushInstructionCache(g_hProcess, (LPCVOID)(uintptr_t)hooks[i].addr, 1);
    }
}

static void disarm_hook_at(unsigned long long addr, unsigned char *orig)
{
    for (int i = 0; i < hook_count; i++) {
        if (hooks[i].addr == addr) {
            SIZE_T n = 0;
            WriteProcessMemory(g_hProcess, (LPVOID)(uintptr_t)addr, &hooks[i].orig, 1, &n);
            FlushInstructionCache(g_hProcess, (LPCVOID)(uintptr_t)addr, 1);
            *orig = hooks[i].orig;
            return;
        }
    }
}

static void rearm_hook(unsigned long long addr)
{
    unsigned char cc = 0xCC;
    SIZE_T n = 0;
    WriteProcessMemory(g_hProcess, (LPVOID)(uintptr_t)addr, &cc, 1, &n);
    FlushInstructionCache(g_hProcess, (LPCVOID)(uintptr_t)addr, 1);
}

static int is_hooked(unsigned long long addr)
{
    for (int i = 0; i < hook_count; i++)
        if (hooks[i].addr == addr)
            return i;
    return -1;
}

static unsigned long long remote_u64(HANDLE hThread, unsigned long long rsp, int off)
{
    unsigned long long v = 0;
    SIZE_T n = 0;
    ReadProcessMemory(g_hProcess, (LPCVOID)(rsp + (unsigned long long)off), &v, sizeof(v), &n);
    return v;
}

static DWORD g_pid = 0;
static volatile LONG g_stop = 0;

static BOOL WINAPI ctrl_handler(DWORD type)
{
    (void)type;
    g_stop = 1;
    fprintf(stderr, "\nstopping... detaching safely\n");
    /* disarm every hook first so the target never hits a stray int3 */
    for (int i = 0; i < hook_count; i++) {
        SIZE_T n = 0;
        WriteProcessMemory(g_hProcess, (LPVOID)(uintptr_t)hooks[i].addr,
                           &hooks[i].orig, 1, &n);
        FlushInstructionCache(g_hProcess, (LPCVOID)(uintptr_t)hooks[i].addr, 1);
    }
    DebugActiveProcessStop(g_pid);
    return TRUE;
}

int main(int argc, char **argv)
{
    if (argc < 3) {
        fprintf(stderr, "usage: tuf_trace.exe <pid> <outbase> [all]\n");
        return 1;
    }
    DWORD pid = (DWORD)strtoul(argv[1], NULL, 0);
    g_pid = pid;
    SetConsoleCtrlHandler(ctrl_handler, TRUE);
    strncpy(g_outbase, argv[2], MAX_PATH - 1);
    g_log_all = (argc > 3 && strcmp(argv[3], "all") == 0);

    g_hProcess = OpenProcess(PROCESS_ALL_ACCESS, FALSE, pid);
    if (!g_hProcess) {
        fprintf(stderr, "OpenProcess(%lu) failed: %lu\n", pid, GetLastError());
        return 1;
    }

    char exe[MAX_PATH * 2] = {0};
    DWORD exe_len = MAX_PATH;
    QueryFullProcessImageNameA(g_hProcess, 0, exe, &exe_len);
    fprintf(stderr, "target: %s (pid %lu)\n", exe, pid);

    /* locate ntdll base in the debuggee via module enumeration */
    HMODULE mods[1024]; DWORD needed = 0;
    if (!EnumProcessModulesEx(g_hProcess, mods, sizeof(mods), &needed, LIST_MODULES_ALL)) {
        fprintf(stderr, "EnumProcessModulesEx failed: %lu\n", GetLastError());
        return 1;
    }
    int nmods = needed / sizeof(HMODULE);
    unsigned long long ntdll_base = 0;
    char ntdll_path[MAX_PATH] = {0};
    for (int i = 0; i < nmods; i++) {
        char name[MAX_PATH] = {0};
        GetModuleFileNameExA(g_hProcess, mods[i], name, MAX_PATH);
        if (strstr(_strlwr(name), "ntdll.dll")) {
            ntdll_base = (unsigned long long)(uintptr_t)mods[i];
            strncpy(ntdll_path, name, MAX_PATH - 1);
            break;
        }
    }
    if (!ntdll_base) { fprintf(stderr, "ntdll not found in target\n"); return 1; }

    /* our own ntdll: same binary → offset math works for the target */
    HMODULE local_nt = GetModuleHandleA("ntdll.dll");
    unsigned long long local_base = (unsigned long long)(uintptr_t)local_nt;
    unsigned long long dc  = local_base + ((unsigned long long)(uintptr_t)GetProcAddress(local_nt, "DeviceIoControl") - local_base);
    unsigned long long ndc = local_base + ((unsigned long long)(uintptr_t)GetProcAddress(local_nt, "NtDeviceIoControlFile") - local_base);
    unsigned long long tdc  = ntdll_base + (dc - local_base);
    unsigned long long tndc = ntdll_base + (ndc - local_base);
    fprintf(stderr, "ntdll base: %llx | DeviceIoControl: %llx | NtDeviceIoControlFile: %llx\n",
            ntdll_base, tdc, tndc);

    char csvpath[MAX_PATH], bufpath[MAX_PATH];
    snprintf(csvpath, MAX_PATH, "%s.csv", g_outbase);
    snprintf(bufpath, MAX_PATH, "%s.buffers.bin", g_outbase);
    g_csv = fopen(csvpath, "w");
    g_buf = fopen(bufpath, "w+b");
    if (!g_csv || !g_buf) { fprintf(stderr, "cannot open output files\n"); return 1; }
    fprintf(g_csv, "ts,tid,handle,ioctl,in_len,out_len,buf_off,status\n");

    put_breakpoint(tdc, "DeviceIoControl");
    put_breakpoint(tndc, "NtDeviceIoControlFile");

    if (!DebugActiveProcess(pid)) {
        fprintf(stderr, "DebugActiveProcess failed: %lu\n", GetLastError());
        return 1;
    }
    DebugSetProcessKillOnExit(FALSE);
    /* the attach raises an initial breakpoint; consume it */
    DEBUG_EVENT ev;
    WaitForDebugEvent(&ev, INFINITE);
    ContinueDebugEvent(ev.dwProcessId, ev.dwThreadId, DBG_CONTINUE);
    arm_all();
    fprintf(stderr, "tracing... (stop with Ctrl+C; trace: %s)\n", csvpath);

    int running = 1;
    while (running) {
        if (g_stop) break;
        if (!WaitForDebugEvent(&ev, 1000)) {
            /* check target still alive */
            if (WaitForSingleObject(g_hProcess, 0) == WAIT_OBJECT_0) {
                fprintf(stderr, "target exited\n");
                break;
            }
            continue;
        }
        DWORD cont = DBG_CONTINUE;
        switch (ev.dwDebugEventCode) {
        case EXCEPTION_DEBUG_EVENT: {
            unsigned long long addr = (unsigned long long)(uintptr_t)ev.u.Exception.ExceptionRecord.ExceptionAddress;
            int hk = is_hooked(addr);
            if (ev.u.Exception.ExceptionRecord.ExceptionCode == EXCEPTION_BREAKPOINT
                && ev.u.Exception.dwFirstChance && hk >= 0) {
                HANDLE hThread = OpenThread(THREAD_ALL_ACCESS, FALSE, ev.dwThreadId);
                if (!hThread) { ContinueDebugEvent(ev.dwProcessId, ev.dwThreadId, DBG_CONTINUE); continue; }
                CONTEXT ctx = { .ContextFlags = CONTEXT_FULL };
                GetThreadContext(hThread, &ctx);

                unsigned long long ioctl = 0, inlen = 0, outlen = 0;
                unsigned long long inbuf = 0, outbuf = 0, handle = 0;
                if (hk == 0) { /* DeviceIoControl: rcx=h, rdx=ioctl, r8=in, r9=inlen, [rsp+0x28]=out, [rsp+0x30]=outlen */
                    handle = ctx.Rcx; ioctl = ctx.Rdx; inbuf = ctx.R8; inlen = ctx.R9;
                    outbuf = remote_u64(hThread, ctx.Rsp, 0x28);
                    outlen = remote_u64(hThread, ctx.Rsp, 0x30);
                } else {      /* NtDeviceIoControlFile: rcx=h, r10=ioctl, r8=in, r9=inlen, [rsp+0x28]=out, [rsp+0x30]=outlen */
                    handle = ctx.Rcx; ioctl = ctx.R10; inbuf = ctx.R8; inlen = ctx.R9;
                    outbuf = remote_u64(hThread, ctx.Rsp, 0x28);
                    outlen = remote_u64(hThread, ctx.Rsp, 0x30);
                }

                int interesting = g_log_all;
                unsigned int fam[7] = {0x22B80A, 0x22B807, 0x22BB07, 0x22BB0A, 0x22C115, 0x226C15, 0x22B015};
                for (int i = 0; i < 7; i++)
                    if ((unsigned int)ioctl == fam[i]) interesting = 1;

                if (interesting && inbuf && inlen) {
                    unsigned char tmp[512];
                    unsigned long long take = inlen < sizeof(tmp) ? inlen : sizeof(tmp);
                    unsigned char *buf = malloc(take ? take : 1);
                    SIZE_T n = 0;
                    ReadProcessMemory(g_hProcess, (LPCVOID)(uintptr_t)inbuf, buf, take, &n);

                    FILETIME ft; GetSystemTimeAsFileTime(&ft);
                    unsigned long long ts = ((unsigned long long)ft.dwHighDateTime << 32) | ft.dwLowDateTime;

                    unsigned long long off = g_buf_off;
                    if (n) { fseek(g_buf, (long)g_buf_off, SEEK_SET); fwrite(buf, 1, n, g_buf); g_buf_off += n; }

                    fprintf(g_csv, "%llu,%lu,%llu,%llu,%llu,%llu,%llu,pre,%lu\n",
                            ts, (unsigned long)ev.dwThreadId, handle, ioctl, inlen, outlen, off, (unsigned long)pid);
                    fflush(g_csv);
                    free(buf);
                    (void)outbuf;
                }

                /* step over: restore byte, TF, continue; re-arm on single-step */
                unsigned char orig;
                disarm_hook_at(addr, &orig);
                ctx.Rip = addr; /* back to the real instruction */
                ctx.EFlags |= 0x100;
                ctx.ContextFlags = CONTEXT_CONTROL;
                SetThreadContext(hThread, &ctx);
                cont = DBG_CONTINUE;
                ContinueDebugEvent(ev.dwProcessId, ev.dwThreadId, cont);
                /* wait for the single-step and re-arm */
                DEBUG_EVENT ss;
                WaitForDebugEvent(&ss, INFINITE);
                if (ss.dwDebugEventCode == EXCEPTION_DEBUG_EVENT &&
                    ss.u.Exception.ExceptionRecord.ExceptionCode == EXCEPTION_SINGLE_STEP) {
                    rearm_hook(addr);
                    ContinueDebugEvent(ss.dwProcessId, ss.dwThreadId, DBG_CONTINUE);
                } else {
                    ContinueDebugEvent(ss.dwProcessId, ss.dwThreadId, DBG_CONTINUE);
                }
                CloseHandle(hThread);
                continue; /* already continued */
            }
            if (ev.u.Exception.ExceptionRecord.ExceptionCode == EXCEPTION_BREAKPOINT) {
                cont = DBG_CONTINUE; /* system breakpoint */
            } else {
                cont = DBG_EXCEPTION_NOT_HANDLED;
            }
            break;
        }
        case EXIT_PROCESS_DEBUG_EVENT:
            running = 0;
            break;
        }
        if (running)
            ContinueDebugEvent(ev.dwProcessId, ev.dwThreadId, cont);
    }

    fclose(g_csv);
    fclose(g_buf);
    fprintf(stderr, "trace complete: %s + %s\n", csvpath, bufpath);
    return 0;
}
