import ctypes
import sys
import threading
import time

# Node-style raw input: VT input on, processed/line/echo input off. This is the
# mode interactive CLI agents use, so their cursor queries must be answered with
# raw VT bytes on stdin.
k = ctypes.windll.kernel32
k.CreateFileW.restype = ctypes.c_void_p
k.CreateFileW.argtypes = [
    ctypes.c_wchar_p,
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_void_p,
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_void_p,
]
k.ReadFile.restype = ctypes.c_int
k.ReadFile.argtypes = [
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_uint32,
    ctypes.c_void_p,
    ctypes.c_void_p,
]

h = k.CreateFileW("\\\\.\\CONIN$", 0xC0000000, 3, None, 3, 0, None)
if not h:
    sys.stdout.write("PROBE_NO_CONIN")
    sys.stdout.flush()
    sys.exit(1)
if not k.SetConsoleMode(ctypes.c_void_p(h), ctypes.c_uint32(0x0200)):
    sys.stdout.write("PROBE_NO_VT_INPUT")
    sys.stdout.flush()
    sys.exit(1)
sys.stdout.write("PROBE_READY>")
sys.stdout.flush()

captured = bytearray()
stop = threading.Event()


def reader():
    while not stop.is_set():
        buf = ctypes.create_string_buffer(256)
        n = ctypes.c_uint32(0)
        if not k.ReadFile(ctypes.c_void_p(h), buf, 256, ctypes.byref(n), None):
            return
        captured.extend(buf.raw[: n.value])


t = threading.Thread(target=reader, daemon=True)
t.start()

sys.stdout.write("\x1b[?6n" if "private" in sys.argv else "\x1b[6n")
sys.stdout.flush()
time.sleep(2)
sys.stdout.write("\x1b[?6n" if "private" in sys.argv else "\x1b[6n")
sys.stdout.flush()
time.sleep(2)
stop.set()
sys.stdout.write("\nGOT:" + bytes(captured).hex(" ") + "\n")
sys.stdout.flush()
time.sleep(3)
