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

## 9. After the wipe: reinstalling an operating system

A wipe erases the **whole** drive: Windows, every partition, the maker's recovery partition and all files. Afterwards the laptop shows "no bootable device" until you install an operating system (OS). atw doesn't do this; it's a separate job.

Only reinstall after the wipe **passed**. If it failed, don't install anything: deal with the failure first (section 10).

Commands marked *(untested)* haven't been tried on a real laptop yet.

### 9.1 Before the wipe: note the Windows licence

Most laptops from about 2015 onwards store their Windows key in the firmware (the laptop's built-in settings chip), not on the drive, so the wipe doesn't remove it. To check, while the atw USB is running, type:

```
ls /sys/firmware/acpi/tables/MSDM        # (untested)
```

- If it prints the file name, the key is in the firmware. Windows will activate by itself after the reinstall.
- If it says "No such file or directory", there's no key in the firmware. Look for a licence sticker (Certificate of Authenticity) on the base or under the battery, and write the key on the intake sheet.

Also note which edition it had (Home or Pro), if the sticker or intake sheet says.

### 9.2 Pick the operating system

| Laptop | Install |
| --- | --- |
| Meets the Windows 11 requirements (roughly: Intel 8th gen or AMD Ryzen 2000 or newer, TPM 2.0) | Windows 11 (section 9.3) |
| Older than that | Windows 10 support ended on 14 Oct 2025, so it gets no more security updates. Install ChromeOS Flex or Linux instead (section 9.4), or tell the buyer clearly. |
| Apple | See section 11 |

### 9.3 Reinstalling Windows 11

**Make a Windows install USB (once; reuse it for every laptop)**

1. You need a separate USB stick of at least 8 GB (not the atw stick). It will be erased.
2. On a Windows computer, go to `https://www.microsoft.com/software-download/windows11` and download the **Media Creation Tool** under "Create Windows 11 Installation Media".
3. Run it, accept the licence, keep the suggested language and edition, choose **USB flash drive**, and pick the stick. Wait until it says it's ready.

Every few months, make the stick again so new installs need fewer updates.

**Install**

1. Plug in the Windows stick (not the atw stick) and turn the laptop on. Open the one-time boot menu (often F12, F9 or Esc) and pick the stick. Leave Secure Boot on: Windows needs it.
2. Choose language and keyboard (English (Australia), US keyboard) and click **Next**, then **Install Windows 11**.
3. **Product key:**
   - Key in the firmware (9.1): setup usually skips this page. If it asks, click **I don't have a product key**.
   - Sticker key: type it in. Microsoft stopped accepting Windows 7 and 8 keys in 2023, so an old sticker may not work; the laptop then needs a new licence.
4. **Edition:** if asked, choose the same one the laptop had (Home or Pro). The wrong one won't activate.
5. Accept the licence terms. If asked, choose **Custom: Install Windows only**.
6. **Where to install:** you should see one drive with only "Unallocated Space", the size of the drive you wiped. Select it and click **Next**. Windows creates its own partitions.
   - **No drives shown** ("We couldn't find any drives"): common on newer Intel laptops. The drive is behind Intel's storage controller (VMD), which needs a driver. On another computer, download the "Intel Rapid Storage Technology (VMD/F6) driver" for that exact model from the maker's support site, unzip it onto the Windows stick, then click **Load driver** and browse to it.
   - **More than one drive:** the laptop has two internal drives. Both should have been wiped. Install to the bigger, faster one (normally NVMe).
7. Wait. The laptop restarts a few times. Take the Windows stick out when the screen first says "Getting ready" or restarts.

**Stop at the first setup screen (leave it for the buyer)**

When the screen asks **"Is this the right country or region?"**, the install is done. Don't sign in or create an account: the buyer does that. You now have two choices:

- **Quick:** press **Shift+F10** to open a command window, type `shutdown /s /t 0` and press Enter *(untested)*. The buyer starts from the country screen.
- **Check drivers first (recommended):** press **Ctrl+Shift+F3**. The laptop restarts into "audit mode", a special administrator desktop that doesn't create an account *(untested)*.
  1. Connect to Wi-Fi. Open **Settings → Windows Update** and install everything, including **Advanced options → Optional updates → Driver updates**. Restart when asked (it comes back to audit mode).
  2. Right-click **Start → Device Manager**. Nothing should have a yellow warning sign. If something does, get the driver from the maker's support site.
  3. Check **Settings → System → Activation** says **Windows is activated**.
  4. Test the basics: Wi-Fi, sound, webcam, keyboard, touchpad, USB ports, charging.
  5. In the **System Preparation Tool** window (it opens by itself in audit mode), set **System Cleanup Action** to **Enter System Out-of-Box Experience (OOBE)**, leave **Generalize** unticked, set **Shutdown Options** to **Shutdown**, and click **OK**.

The laptop turns off. When the buyer turns it on, they get the normal Windows setup from the start.

### 9.4 ChromeOS Flex or Linux (older laptops)

- **ChromeOS Flex** (free, from Google; simple for buyers): in Chrome on any computer, add the **Chromebook Recovery Utility** extension, choose **Google ChromeOS Flex**, and make a USB stick. Boot the laptop from it and choose **Install ChromeOS Flex**. Check Google's certified models list first; most business laptops from about 2010 onwards work.
- **Linux Mint** (free; looks like Windows): download the ISO from `linuxmint.com`, write it to a USB stick with balenaEtcher, boot from it and choose **Install Linux Mint**. Choose **Erase disk and install**. It works with Secure Boot on.

### 9.5 Before it goes on sale

- The OS starts and is activated (Windows), or setup starts from the beginning for the buyer.
- No personal accounts are signed in, yours or anyone else's.
- The intake record shows the wipe passed. The certificate is issued after sync (section 12).

## 10. Failure messages *(coming)*
## 11. Apple devices: manual erase *(coming)*
## 12. Syncing and issuing certificates *(coming)*
## 13. If a USB is lost *(coming)*
