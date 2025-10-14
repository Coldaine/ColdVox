#!/bin/bash
set -euo pipefail

# This script orchestrates the VM-based testing for a given compositor.

# --- Configuration ---
COMPOSITOR="$1"
TEST_BINARY_PATH="$2"
BASE_IMAGE_DIR="/var/lib/libvirt/images"
BASE_IMAGE_NAME="coldvox-${COMPOSITOR}.qcow2"
BASE_IMAGE_PATH="${BASE_IMAGE_DIR}/${BASE_IMAGE_NAME}"
VM_XML_DIR="testing-infra/vms"
RESULTS_DIR="results"

# --- Dynamic variables ---
VM_NAME="coldvox-test-${COMPOSITOR}-${GITHUB_RUN_ID:-local}"
SNAPSHOT_IMAGE_PATH="${BASE_IMAGE_DIR}/${VM_NAME}.qcow2"
VM_XML_TEMPLATE_PATH="${VM_XML_DIR}/${COMPOSITOR}.xml"
VM_XML_PATH="/tmp/${VM_NAME}.xml"

# --- SSH Configuration ---
SSH_USER="testuser"
SSH_KEY_PATH="/tmp/id_testvm" # This will be created from a GH secret
SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i ${SSH_KEY_PATH}"

# --- Functions ---
cleanup() {
  echo "--- Cleaning up VM: ${VM_NAME} ---"
  virsh destroy "${VM_NAME}" || true
  rm -f "${SNAPSHOT_IMAGE_PATH}"
  rm -f "${VM_XML_PATH}"
  rm -f "${SSH_KEY_PATH}"
}

# --- Main execution ---
trap cleanup EXIT

# 0. Setup SSH key from secret
# In a real CI run, this would be populated from a GitHub secret
# For local testing, you can create a key pair and place the private key here
if [[ -z "${CI_SSH_KEY:-}" ]]; then
  echo "Warning: CI_SSH_KEY secret not set. Using a placeholder key."
  # This is insecure and only for local testing.
  ssh-keygen -t rsa -b 2048 -f "${SSH_KEY_PATH}" -N ""
  # The public key must be in the Packer-built image's authorized_keys
else
  echo "${CI_SSH_KEY}" > "${SSH_KEY_PATH}"
fi
chmod 600 "${SSH_KEY_PATH}"

# 1. Check for base image and test binary
if [[ ! -f "${BASE_IMAGE_PATH}" ]]; then
  echo "Error: Base image not found at ${BASE_IMAGE_PATH}" >&2
  exit 1
fi
if [[ ! -f "${TEST_BINARY_PATH}" ]]; then
  echo "Error: Test binary not found at ${TEST_BINARY_PATH}" >&2
  exit 1
fi

# 2. Create snapshot
echo "--- Creating snapshot for ${VM_NAME} ---"
qemu-img create -f qcow2 -b "${BASE_IMAGE_PATH}" -F qcow2 "${SNAPSHOT_IMAGE_PATH}"

# 3. Define and start VM
echo "--- Starting VM: ${VM_NAME} ---"
sed "s|__VM_NAME__|${VM_NAME}|g" "${VM_XML_TEMPLATE_PATH}" | \
sed "s|__SNAPSHOT_IMAGE_PATH__|${SNAPSHOT_IMAGE_PATH}|g" > "${VM_XML_PATH}"

virsh define "${VM_XML_PATH}"
virsh start "${VM_NAME}"

# 4. Wait for SSH to be ready
echo "--- Waiting for VM to boot and get an IP ---"
VM_IP=""
for i in {1..60}; do
  VM_IP=$(virsh domifaddr "${VM_NAME}" --source lease | awk '/ipv4/ {print $4}' | cut -d'/' -f1)
  if [[ -n "${VM_IP}" ]]; then
    break
  fi
  echo "Still waiting for IP..."
  sleep 5
done

if [[ -z "${VM_IP}" ]]; then
  echo "Error: Could not get VM IP address after 5 minutes." >&2
  virsh domifaddr "${VM_NAME}" --source lease
  exit 1
fi
echo "VM IP found: ${VM_IP}"

echo "--- Waiting for SSH daemon to become available ---"
for i in {1..24}; do
    if ssh ${SSH_OPTS} "${SSH_USER}@${VM_IP}" "echo 'SSH is ready'" &> /dev/null; then
        echo "SSH is ready."
        break
    fi
    echo "Waiting for SSH..."
    sleep 5
done

# 5. Deploy and execute tests
echo "--- Deploying test assets to ${VM_NAME} ---"
scp ${SSH_OPTS} "testing-infra/scripts/vm-run-tests.sh" "${SSH_USER}@${VM_IP}:~/"
scp ${SSH_OPTS} "${TEST_BINARY_PATH}" "${SSH_USER}@${VM_IP}:~/coldvox-text-injection"
ssh ${SSH_OPTS} "${SSH_USER}@${VM_IP}" "chmod +x vm-run-tests.sh coldvox-text-injection"

echo "--- Executing test suite in ${VM_NAME} ---"
if ! ssh ${SSH_OPTS} "${SSH_USER}@${VM_IP}" "./vm-run-tests.sh"; then
  echo "Error: Test suite failed in ${VM_NAME}" >&2
  # We still proceed to collect results
fi

# 6. Collect results
echo "--- Collecting results from ${VM_NAME} ---"
mkdir -p "${RESULTS_DIR}/${COMPOSITOR}"
if ! scp ${SSH_OPTS} "${SSH_USER}@${VM_IP}:/tmp/results.tar.gz" "${RESULTS_DIR}/${COMPOSITOR}/"; then
    echo "Warning: Failed to collect results archive." >&2
fi

echo "--- Test run for ${COMPOSITOR} complete ---"