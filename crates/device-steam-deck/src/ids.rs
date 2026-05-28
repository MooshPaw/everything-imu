//! Vendor + product IDs for the Steam Deck integrated controller.

pub const VALVE_VID: u16 = 0x28DE;

/// Same PID for LCD (jupiter) and OLED (galileo) Decks. Three USB interfaces
/// enumerate under this PID — the gamepad, an emulated mouse, and an emulated
/// keyboard. Only the gamepad interface accepts HID feature reports.
pub const STEAM_DECK_PID: u16 = 0x1205;

/// HID usage page + usage that identifies the gamepad interface (the other two
/// interfaces are keyboard + mouse usage pages).
pub const GAMEPAD_USAGE_PAGE: u16 = 0xFFFF;
pub const GAMEPAD_USAGE: u16 = 0x0001;

/// Returns true if a hidapi `DeviceInfo` (vid, pid, usage_page, usage) tuple is
/// the Steam Deck gamepad interface and not one of the emulated peripherals.
pub fn is_gamepad_interface(vid: u16, pid: u16, usage_page: u16, usage: u16) -> bool {
    if vid != VALVE_VID || pid != STEAM_DECK_PID {
        return false;
    }
    // Older hidapi builds report (0, 0) for vendor-defined pages — accept either.
    (usage_page == 0 && usage == 0) || (usage_page == GAMEPAD_USAGE_PAGE && usage == GAMEPAD_USAGE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_gamepad_interface() {
        assert!(is_gamepad_interface(
            VALVE_VID,
            STEAM_DECK_PID,
            GAMEPAD_USAGE_PAGE,
            GAMEPAD_USAGE
        ));
        assert!(is_gamepad_interface(VALVE_VID, STEAM_DECK_PID, 0, 0));
    }

    #[test]
    fn rejects_keyboard_mouse_interfaces() {
        // Keyboard usage page = 0x0001, usage = 0x0006
        assert!(!is_gamepad_interface(
            VALVE_VID,
            STEAM_DECK_PID,
            0x0001,
            0x0006
        ));
        // Mouse usage page = 0x0001, usage = 0x0002
        assert!(!is_gamepad_interface(
            VALVE_VID,
            STEAM_DECK_PID,
            0x0001,
            0x0002
        ));
    }

    #[test]
    fn rejects_wrong_vid_pid() {
        assert!(!is_gamepad_interface(0x057E, STEAM_DECK_PID, 0, 0));
        assert!(!is_gamepad_interface(VALVE_VID, 0x1102, 0, 0));
    }
}
