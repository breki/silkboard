#!/usr/bin/env bash
# Bootstrap only: create the named guest, never replace an existing domain/disk.
set -euo pipefail
umask 022
fail() { echo "silkboard VM: $*" >&2; exit 1; }
mode=check
image=
keyfile=
while (($#)); do
    case "$1" in
        --apply) mode=apply; shift ;;
        --check) mode=check; shift ;;
        --image|--ssh-public-key)
            (($# >= 2)) || fail "missing value for $1"
            if [[ $1 == --image ]]; then image=$2; else keyfile=$2; fi
            shift 2 ;;
        *) fail "usage: $0 [--check|--apply] --image FILE --ssh-public-key FILE" ;;
    esac
done
[[ $EUID == 0 && $(hostname -s) == malina && $(uname -m) == aarch64 ]] ||
    fail 'run as root on malina ARM64'
[[ -f $image && -f $keyfile ]] || fail 'image and public key files required'
for tool in virsh virt-install qemu-img cloud-localds ssh-keygen sha512sum; do
    command -v "$tool" >/dev/null || fail "missing $tool"
done
base=/var/lib/libvirt/images/silkboard
[[ ! -e $base && ! -L $base ]] || fail "$base already exists; inspect, do not overwrite"
# A connection error is not evidence that the domain is absent.
domains=$(virsh --connect qemu:///system list --all --name)
if grep -qx silkboard <<< "$domains"; then fail 'silkboard domain already exists'; fi
available=$(awk '/MemAvailable:/ {print $2}' /proc/meminfo)
((available >= 3145728)) || fail 'need at least 3 GiB host available RAM'
space=$(df -Pk /var/lib/libvirt/images | awk 'NR==2 {print $4}')
((space >= 47185920)) || fail 'need at least 45 GiB free host disk'
expected=a34e1c1eea1fbb9ad31ebdfc6c776d1d579ce9e3d740ab10549ebcdca6fe1a46c28e0a42cbcdf1eb561795776733c178f6e816fe1a2cb60d26ffc56c07f915c4
printf '%s  %s\n' "$expected" "$image" | sha512sum --check --status ||
    fail 'Debian 13 ARM64 image checksum mismatch'
ssh-keygen -lf "$keyfile" >/dev/null
read -r keytype keybody _ < "$keyfile"
[[ $keytype == ssh-ed25519 && $keybody =~ ^[A-Za-z0-9+/=]+$ ]] ||
    fail 'expected one ED25519 SSH public key'
scriptdir=$(cd -- "$(dirname -- "$0")" && pwd)
[[ -f $scriptdir/cloud-init.yaml.in ]] || fail 'missing cloud-init template'
echo 'Verified: new silkboard guest, 2 vCPUs, fixed 2 GiB RAM, 40 GiB disk.'
[[ $mode == apply ]] || exit 0
install -d -m 0755 "$base"
# Independent base: production never depends on retained trial files.
cp --reflink=auto -- "$image" "$base/base.qcow2"
chmod 0444 "$base/base.qcow2"
qemu-img create -f qcow2 -F qcow2 -b "$base/base.qcow2" "$base/root.qcow2" 40G
sed "s|@OPERATOR_KEY@|$keytype $keybody|" "$scriptdir/cloud-init.yaml.in" > "$base/user-data"
printf 'instance-id: silkboard-production-1\nlocal-hostname: silkboard\n' > "$base/meta-data"
cloud-localds "$base/seed.iso" "$base/user-data" "$base/meta-data"
chown -R libvirt-qemu:libvirt-qemu "$base"
virt-install --connect qemu:///system --name silkboard \
    --memory 2048 --vcpus 2 --arch aarch64 --machine virt --virt-type kvm \
    --cpu host-passthrough --import --osinfo debian13 --boot uefi \
    --disk "path=$base/root.qcow2,format=qcow2,bus=virtio" \
    --disk "path=$base/seed.iso,device=cdrom" \
    --network user,model=virtio,backend.type=passt,rom.bar=off \
    --channel unix,target.type=virtio,target.name=org.qemu.guest_agent.0 \
    --memballoon none --graphics none --noautoconsole
virsh --connect qemu:///system autostart silkboard
echo 'Guest created. Use the operator-only guest agent until private SSH is ready.'
