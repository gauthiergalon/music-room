# Oracle Cloud VM

`create-vm.sh` creates the Always Free Ampere VM that runs Music Room, and
keeps retrying while Oracle answers "Out of capacity", which is common for
free VMs.

## One-time setup

1. **Network.** In the console: Networking → Virtual Cloud Networks →
   Start VCN Wizard → *Create VCN with Internet Connectivity*. It creates the
   public subnet the VM goes in. Then open ports 80 and 443: the public
   subnet → its Default Security List → Add Ingress Rules (source
   `0.0.0.0/0`, TCP, destination port 80, then the same for 443).

2. **OCI CLI.** Installed in its own Python environment:

   ```bash
   python3 -m venv ~/.local/share/oci-cli
   ~/.local/share/oci-cli/bin/pip install oci-cli==3.95.1
   ```

3. **API key**, so the CLI can act for your account. In the console: your
   profile (top right) → My profile → API keys (under *Tokens and keys*) →
   Add API key → *Generate API key pair* → **Download private key** → Add.
   Save the key as `~/.oci/oci_api_key.pem`, then copy the *Configuration
   file preview* shown by the console into `~/.oci/config` and set its
   `key_file` line to that path:

   ```bash
   mkdir -p ~/.oci && chmod 700 ~/.oci
   mv ~/Downloads/*.pem ~/.oci/oci_api_key.pem && chmod 600 ~/.oci/oci_api_key.pem
   nano ~/.oci/config          # paste the preview, fix key_file
   chmod 600 ~/.oci/config
   ~/.local/share/oci-cli/bin/oci iam region list   # works = configured
   ```

   The private key gives full access to your Oracle account: keep it out
   of the repository and of any shared folder.

## Creating the VM

```bash
./deploy/oracle/create-vm.sh                       # 2 OCPU / 12 GB
OCPUS=1 MEMORY_GB=6 ./deploy/oracle/create-vm.sh   # smaller, easier to get
```

It tries once a minute and prints each attempt; leave it running (hours,
sometimes days, depending on the region). It stops by itself once the VM
exists and prints the `ssh` command to reach it. It refuses sizes beyond
the Always Free limits (2 OCPU, 12 GB) and does nothing if the VM already
exists. `UBUNTU` (26.04 by default), `NAME`, `SSH_KEY` and `INTERVAL` can be changed in
the environment.

To keep it running after closing the terminal:

```bash
nohup ./deploy/oracle/create-vm.sh > create-vm.log 2>&1 &
tail -f create-vm.log
```
