wget https://cloud-images.ubuntu.com/releases/focal/release/ubuntu-20.04-server-cloudimg-amd64.img
qemu-img resize server.img +20G
genisoimage -output nocloud.iso -volid cidata -joliet -rock user-data meta-data
