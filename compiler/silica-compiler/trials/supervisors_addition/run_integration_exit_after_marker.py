#!/usr/bin/env python3
# Integrate helper: run a trial with stdout/stderr on a PTY (line-oriented stdio) and stdin on a pipe.
# Feed "exit\n" to stdin only after a full line equals <marker> (e.g. supervisor "done") **and** the PTY has
# been quiet for a short idle window (bounded). Some trials (e4f) print `done` before async failure banners;
# sending exit on `done` alone tears down the process early → SIGBUS / `.sout` with only the exit code.
# Appends the process exit code as the final line (same as echo $? in run_integration_binary.sh).
"""Usage: run_integration_exit_after_marker.py <trial_dir> <basename> <out_path> <marker_line>"""

import errno
import os
import pty
import select
import sys
import time


def quiesce_pty(
    master: int, out_f, idle_sec: float = 0.08, cap_sec: float = 3.0
) -> None:
    """Keep reading the PTY until `idle_sec` passes with no readable data, or `cap_sec` total wall time."""
    deadline = time.monotonic() + cap_sec
    while time.monotonic() < deadline:
        timeout = min(idle_sec, deadline - time.monotonic())
        if timeout <= 0:
            break
        r, _, _ = select.select([master], [], [], timeout)
        if r:
            chunk = os.read(master, 65536)
            if not chunk:
                return
            out_f.write(chunk)
        else:
            # select timed out: no data for `timeout` (≤ idle_sec) → trailing output drained
            return


def main() -> int:
    if len(sys.argv) != 5:
        sys.stderr.write(
            "usage: run_integration_exit_after_marker.py "
            "<trial_dir> <basename> <out_path> <marker_line>\n"
        )
        return 2

    trial_dir, base, out_path, marker = sys.argv[1:5]
    exe = os.path.join(trial_dir, base)
    if not (os.path.isfile(exe) and os.access(exe, os.X_OK)):
        sys.stderr.write(f"missing or not executable: {exe}\n")
        return 1

    marker_b = marker.encode("utf-8")

    master, slave = pty.openpty()
    stdin_r, stdin_w = os.pipe()

    pid = os.fork()
    if pid == 0:
        os.close(master)
        os.close(stdin_w)
        try:
            os.setsid()
        except OSError:
            pass
        os.dup2(stdin_r, 0)
        os.dup2(slave, 1)
        os.dup2(slave, 2)
        if stdin_r > 2:
            os.close(stdin_r)
        if slave > 2:
            os.close(slave)
        os.chdir(trial_dir)
        os.execv(exe, [exe])
        return 127

    os.close(stdin_r)
    os.close(slave)

    exit_sent = False
    buf = b""
    try:
        with open(out_path, "wb", buffering=0) as out_f:
            while True:
                r, _, _ = select.select([master], [], [])
                chunk = os.read(master, 65536)
                if not chunk:
                    break
                out_f.write(chunk)
                if not exit_sent:
                    buf += chunk
                    while True:
                        idx = buf.find(b"\n")
                        if idx < 0:
                            break
                        line = buf[:idx].rstrip(b"\r")
                        buf = buf[idx + 1 :]
                        if line == marker_b:
                            quiesce_pty(master, out_f)
                            try:
                                os.write(stdin_w, b"exit\n")
                            except BrokenPipeError:
                                pass
                            except OSError as exc:
                                if exc.errno != errno.EPIPE:
                                    raise
                            exit_sent = True
                            break
    finally:
        try:
            os.close(stdin_w)
        except OSError:
            pass

    _pid, wstatus = os.waitpid(pid, 0)
    try:
        os.close(master)
    except OSError:
        pass

    if os.WIFEXITED(wstatus):
        code = os.WEXITSTATUS(wstatus)
    elif os.WIFSIGNALED(wstatus):
        code = 128 + os.WTERMSIG(wstatus)
    else:
        code = 1

    with open(out_path, "ab", buffering=0) as out_f:
        out_f.write(f"{code}\n".encode("ascii"))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
