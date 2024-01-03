KERNEL=/home/aoli/repos/ghost-kernel
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

qemu-system-x86_64 \
    -kernel $KERNEL/arch/x86/boot/bzImage \
    -enable-kvm -cpu host -smp 8 -m 4096 -nographic \
    -drive id=root,media=disk,file=ubuntu-20.04-server-cloudimg-amd64.img \
    -append "root=/dev/sda1 console=ttyS0" \
    -device e1000,netdev=net0 -netdev user,id=net0,hostfwd=tcp:127.0.0.1:5555-:22 \
    -virtfs local,path=$SCRIPT_DIR,mount_tag=hostshare,security_model=none,id=hostshare
