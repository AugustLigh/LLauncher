# AUR packaging

`PKGBUILD` here is the source of truth for the [AUR](https://aur.archlinux.org) package
`llauncher-bin` (prebuilt binary repackaged from the `.deb` release asset).

Users install it with:

```bash
yay -S llauncher-bin   # or: paru -S llauncher-bin
```

## Automated publishing

`.github/workflows/aur-publish.yml` pushes an updated PKGBUILD to the AUR every time a
GitHub release is published (it can also be run manually via *workflow dispatch* with a
tag name). It bumps `pkgver`, regenerates `sha256sums` and commits to the AUR git repo.

### One-time setup

1. Create an account on <https://aur.archlinux.org> (if you don't have one).
2. Generate a dedicated SSH key pair:
   ```bash
   ssh-keygen -t ed25519 -f aur_key -C "aur@llauncher" -N ""
   ```
3. Add the **public** key (`aur_key.pub`) in AUR → *My Account* → *SSH Public Key*.
4. Create the package base by cloning and pushing once, or let the workflow do the
   first push (the AUR creates the package on first push to
   `ssh://aur@aur.archlinux.org/llauncher-bin.git`).
5. In the GitHub repo, add three Actions secrets (*Settings → Secrets and variables →
   Actions*):
   - `AUR_USERNAME` — your AUR username
   - `AUR_EMAIL` — email for the AUR git commits
   - `AUR_SSH_PRIVATE_KEY` — contents of the **private** key file (`aur_key`)

### Testing locally (on Arch)

```bash
cd packaging/aur
makepkg -si
```
