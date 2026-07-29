#!/bin/bash
# ==================================================================
# Game-Op Intel iGPU Clock Pinning Permission Setup Tool
# ==================================================================
set -e

echo "=================================================================="
echo " ⚙️  Configuring Passwordless Intel iGPU Frequency Controls..."
echo "=================================================================="

# 1. Determine active user
USER_NAME=$(whoami)
if [ "$USER_NAME" = "root" ]; then
    if [ -n "$SUDO_USER" ]; then
        USER_NAME="$SUDO_USER"
    fi
fi

# 2. Check if udev rules directory exists
UDEV_DIR="/etc/udev/rules.d"
if [ ! -d "$UDEV_DIR" ]; then
    echo "⚠️  udev directory $UDEV_DIR not found. Applying local fallback only."
else
    RULE_PATH="$UDEV_DIR/99-intel-gpu-limits.rules"
    echo "Writing udev permission rules to $RULE_PATH..."
    
    # Write rule safely via sudo bash
    sudo bash -c "cat << 'EOF' > $RULE_PATH
# Game-Op: Grant members of the video group write access to Intel iGPU frequency limits
SUBSYSTEM==\"drm\", KERNEL==\"card*\", ACTION==\"add|change\", RUN+=\"/bin/chmod 664 /sys/class/drm/%k/gt_min_freq_mhz /sys/class/drm/%k/gt_max_freq_mhz\", RUN+=\"/bin/chgrp video /sys/class/drm/%k/gt_min_freq_mhz /sys/class/drm/%k/gt_max_freq_mhz\"
EOF"

    echo "Adding user '$USER_NAME' to the 'video' group..."
    if command -v usermod &> /dev/null; then
        sudo usermod -aG video "$USER_NAME"
    elif command -v gpasswd &> /dev/null; then
        sudo gpasswd -a "$USER_NAME" video
    fi

    echo "Reloading udev rules to apply changes..."
    sudo udevadm control --reload-rules
    sudo udevadm trigger
fi

# Correct permissions on existing sysfs nodes immediately
echo "Applying permissions on active sysfs card nodes immediately..."
for card_dir in /sys/class/drm/card[0-9]*; do
    if [ -f "$card_dir/gt_min_freq_mhz" ]; then
        sudo chmod 664 "$card_dir/gt_min_freq_mhz" "$card_dir/gt_max_freq_mhz"
        sudo chgrp video "$card_dir/gt_min_freq_mhz" "$card_dir/gt_max_freq_mhz"
        echo "  ✅ Fixed permissions for: $card_dir"
    fi
done

echo "=================================================================="
echo " 🎉 Success! Intel iGPU limits are now writable by '$USER_NAME'."
echo " 👉 NOTE: Please log out and log back in for group membership to apply!"
echo "=================================================================="
