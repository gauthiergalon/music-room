#!/usr/bin/env bash
# Creates the Always Free Ampere (ARM) VM on Oracle Cloud, retrying until
# Oracle has capacity: "Out of capacity" is common for these free VMs.
#
#   ./deploy/oracle/create-vm.sh
#   OCPUS=1 MEMORY_GB=6 ./deploy/oracle/create-vm.sh    # smaller, easier to get
#
# Needs the OCI CLI configured for your account (~/.oci/config, see
# deploy/oracle/README.md). Everything else is found automatically: the
# availability domain, the latest Ubuntu image for ARM, and the public
# subnet created by the VCN wizard.
set -euo pipefail

OCI=${OCI:-$(command -v oci || echo "$HOME/.local/share/oci-cli/bin/oci")}
NAME=${NAME:-music-room-server}
OCPUS=${OCPUS:-2}
MEMORY_GB=${MEMORY_GB:-12}
SSH_KEY=${SSH_KEY:-$HOME/.ssh/id_ed25519.pub}
UBUNTU=${UBUNTU:-26.04}
INTERVAL=${INTERVAL:-60}   # seconds between two attempts

# Always Free: 2 OCPUs and 12 GB in total for Ampere VMs. Beyond that, the
# VM is paid (Pay As You Go) or deleted at the end of the trial.
if (( OCPUS > 2 || MEMORY_GB > 12 )); then
  echo "Always Free allows at most 2 OCPUs and 12 GB for Ampere VMs." >&2
  exit 1
fi

# A new API key takes a few minutes to be known by every Oracle server:
# until then some calls fail with NotAuthenticated. Retry those for a while.
oci_call() {
  local output
  for _ in 1 2 3 4 5 6; do
    if output=$("$OCI" "$@" 2>&1); then printf '%s\n' "$output"; return 0; fi
    [[ $output == *NotAuthenticated* ]] || break
    sleep 20
  done
  printf '%s\n' "$output" >&2
  return 1
}

tenancy=$(sed -n 's/^tenancy *= *//p' ~/.oci/config | head -1)
compartment=${COMPARTMENT:-$tenancy}

# Never create a second VM by running this again after a success.
existing=$(oci_call compute instance list --compartment-id "$compartment" --display-name "$NAME" \
  --query 'data[?"lifecycle-state"!=`TERMINATED`] | [0].id' --raw-output)
if [ -n "$existing" ]; then
  echo "A VM named $NAME already exists: $existing" >&2
  exit 1
fi

ad=$(oci_call iam availability-domain list --compartment-id "$tenancy" \
  --query 'data[0].name' --raw-output)
image=$(oci_call compute image list --compartment-id "$compartment" \
  --operating-system "Canonical Ubuntu" --operating-system-version "$UBUNTU" \
  --shape VM.Standard.A1.Flex --sort-by TIMECREATED --sort-order DESC \
  --query 'data[0].id' --raw-output)
subnet=${SUBNET_ID:-$(oci_call network subnet list --compartment-id "$compartment" \
  --query 'data[?"prohibit-public-ip-on-vnic"==`false`] | [0].id' --raw-output)}

if [ -z "$image" ] || [ -z "$subnet" ]; then
  echo "No Ubuntu $UBUNTU ARM image or no public subnet found (image: '$image', subnet: '$subnet')." >&2
  echo "Create the network first with the VCN wizard (Create VCN with Internet Connectivity)." >&2
  exit 1
fi

echo "Creating $NAME: $OCPUS OCPU, $MEMORY_GB GB, in $ad"
attempt=0
auth_failures=0
while true; do
  attempt=$((attempt + 1))
  # --no-retry: "out of capacity" is an error 500, which the CLI would
  # otherwise retry itself for a minute; this loop sets the pace instead.
  if output=$("$OCI" compute instance launch \
      --compartment-id "$compartment" --availability-domain "$ad" \
      --display-name "$NAME" --shape VM.Standard.A1.Flex \
      --shape-config "{\"ocpus\": $OCPUS, \"memoryInGBs\": $MEMORY_GB}" \
      --image-id "$image" --subnet-id "$subnet" --assign-public-ip true \
      --boot-volume-size-in-gbs 50 --ssh-authorized-keys-file "$SSH_KEY" \
      --query 'data.id' --raw-output --no-retry 2>&1); then
    instance=$output
    break
  fi
  [[ $output == *NotAuthenticated* ]] || auth_failures=0

  case "$output" in
    *[Oo]"ut of host capacity"*|*[Oo]"ut of capacity"*|*InternalError*)
      wait=$INTERVAL; reason="no capacity" ;;
    *TooManyRequests*)
      wait=$(( INTERVAL * 3 )); reason="too many requests" ;;
    *NotAuthenticated*)
      # A new API key takes a few minutes to be known everywhere; a wrong
      # configuration keeps failing.
      auth_failures=$((auth_failures + 1))
      if (( auth_failures > 5 )); then echo "$output" >&2; exit 1; fi
      wait=$INTERVAL; reason="not authenticated yet ($auth_failures/5)" ;;
    *)
      echo "$output" >&2; exit 1 ;;
  esac
  echo "$(date +%H:%M:%S) attempt $attempt: $reason, next try in ${wait}s"
  sleep "$wait"
done

echo "Got it after $attempt attempts: $instance"
oci_call compute instance get --instance-id "$instance" --wait-for-state RUNNING >/dev/null
ip=$(oci_call compute instance list-vnics --instance-id "$instance" \
  --query 'data[0]."public-ip"' --raw-output)
echo "Running. Connect with: ssh ubuntu@$ip"
