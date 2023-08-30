import subprocess
import sys
import os


p = subprocess.Popen(sys.argv[1:])
print(p.pid)
if os.path.exists("/sys/fs/ghost/enclave_1/tasks"):
    with open("/sys/fs/ghost/enclave_1/tasks", "w") as f:
        f.write(str(p.pid))
        f.flush()
p.communicate()
