These builds are not signed. Windows and macOS warn about an unknown developer the first time;
the program itself is unchanged by that. Check the download against `SHA256SUMS` first
(`sha256sum -c SHA256SUMS --ignore-missing`, on macOS `shasum -a 256 -c SHA256SUMS --ignore-missing`).

**Windows:** unpack the zip, double-click `tracyo.exe`. If SmartScreen says "Windows protected
your PC", click "More info", then "Run anyway".

**macOS:** unpack the archive. macOS marks files from the internet and refuses the program; remove
the mark once in a Terminal in the unpacked folder: `xattr -dr com.apple.quarantine tracyo`, then
start it with `./tracyo` or by double-click. Choose the arm64 archive on Apple silicon and the
x86_64 archive on an Intel Mac.

**Linux:** unpack, run `./tracyo`. The file dialogs need `xdg-desktop-portal` with a backend (for
example `xdg-desktop-portal-gtk`); without one, pass the picture on the command line or drop it on
the window.

Each archive holds the program, `LICENSE` (AGPL-3.0-or-later), `THIRD-PARTY.md` and the licence
files of the bundled font, icons and text recognition model.
