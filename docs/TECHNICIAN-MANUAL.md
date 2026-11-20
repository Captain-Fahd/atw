# atw technician manual

Plain steps for using the AccessibleTech Wipe USB at the bench. Sections marked *(coming)* are filled in as each phase is built.

## 1. Making a wipe USB

You need a USB stick of at least 4 GB. Everything on it will be erased.

1. On the build computer, open a terminal in the `atw` folder and run `nix develop`, then `image/build.sh`. This makes `build/atw-usb.img`.
2. Plug in the stick. Run `lsblk -o NAME,TRAN,SIZE,MODEL,SERIAL` and find the stick: TRAN says `usb` and the size matches. Note its name, for example `sdb`.
3. Run `sudo image/make-usb.sh /dev/sdb` (use your stick's name).
4. The script shows the stick's details and asks you to type its serial number. Type it exactly. If it doesn't match, nothing is written.
5. Wait for "Done". Unplug the stick.

The script refuses anything that isn't a removable USB stick, so it can't touch the computer's own drives.

## 2. Preparing a laptop *(coming)*

## 3. Booting the USB (Secure Boot on)

1. Turn the laptop off. Plug in the wipe USB.
2. Turn it on and open the one-time boot menu (often F12, F9 or Esc; it depends on the brand).
3. Pick the USB stick (it may be listed as "UEFI: <stick name>"). Leave Secure Boot on.
4. A menu appears for 5 seconds, then the USB starts by itself. Wait for the text screen.

## 4. Running probe (Phase 1 testing)

Probe only reads. It doesn't change anything on the laptop.

1. After the USB starts, probe runs by itself and prints a summary:
   - **Secure Boot**: should say `yes`.
   - **lockdown**: should say `integrity` (the kernel's protection mode while Secure Boot is on).
   - One line per drive. Internal drives should show **passthrough: yes**, which means the drive answered the tool's questions.
2. It saves the full results to the USB, in the `probe` folder of the ATECH-DATA partition, and prints the file name.
3. To run it again, type `atw probe` and press Enter. To finish, type `poweroff`.
4. Back at the build computer, copy the files from the stick's `probe` folder to `docs/hw/phase1/` in the repo.

If it says "ATECH-DATA partition not found", the results went to the laptop's memory only and will be lost at shutdown. Remake the USB.

## 5. Entering the PIN *(coming)*
## 6. Intake ID *(coming)*
## 7. Confirming the wipe *(coming)*
## 8. Reading the results *(coming)*
## 9. Failure messages *(coming)*
## 10. Apple devices: manual erase *(coming)*
## 11. Syncing and issuing certificates *(coming)*
## 12. If a USB is lost *(coming)*
