import subprocess
import sys


p = subprocess.Popen(sys.argv[1:])
print(p.pid)
with open("/sys/fs/ghost/enclave_1/tasks", "w") as f:
    f.write(str(p.pid))
    f.flush()
p.communicate()