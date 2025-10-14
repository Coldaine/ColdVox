# packer/fedora-xfce.pkr.hcl

packer {
  required_plugins {
    qemu = {
      version = ">= 1.0.0"
      source  = "github.com/hashicorp/qemu"
    }
  }
}

variable "image_name" {
  type    = string
  default = "coldvox-fedora-xfce"
}

variable "image_version" {
  type    = string
  default = "1.0.0"
}

source "qemu" "fedora-xfce" {
  # VM settings
  iso_url            = "https://download.fedoraproject.org/pub/fedora/linux/releases/40/Spins/x86_64/iso/Fedora-Xfce-Live-x86_64-40-1.14.iso"
  iso_checksum       = "sha256:d8f2375836a43d9943f96d595232757523f290d2a8435d88a12613e551061994"
  output_directory   = "output/${var.image_name}"
  vm_name            = "${var.image_name}-${var.image_version}.qcow2"

  # QEMU settings
  accelerator        = "kvm"
  cpus               = 2
  memory             = 4096
  disk_size          = "20G"
  format             = "qcow2"
  headless           = true

  # Boot command to automate the installation
  boot_command = [
    "<wait><enter><wait10>",
    "liveinst --repo=cdrom --ostree-repo=/run/ostree/repo --disk=/dev/vda --noverifyssl --min-size=10240 --nompath --vncpassword=password --no-user-interaction --reboot<enter>"
  ]

  # SSH settings for provisioning
  ssh_username       = "testuser"
  ssh_password       = "password"
  ssh_timeout        = "20m"
  shutdown_command   = "sudo /sbin/halt -p"
}

build {
  sources = ["source.qemu.fedora-xfce"]

  provisioner "shell" {
    scripts = [
      "../scripts/base-setup.sh",
      "../scripts/compositor-setup.sh",
      "../scripts/polkit-rules.sh"
    ]
  }
}