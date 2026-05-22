import { create } from "zustand";

/**
 * OSC addresses the haptic bridge has reported seeing from VRChat. The
 * Haptics page reads this so the user can bind a contact parameter by
 * tapping it in-game and clicking "Use".
 */
type State = {
  discovered: string[];
  add(address: string): void;
  clear(): void;
};

const MAX_DISCOVERED = 200;

export const useHapticStore = create<State>((set) => ({
  discovered: [],
  add: (address) =>
    set((s) =>
      s.discovered.includes(address)
        ? s
        : { discovered: [address, ...s.discovered].slice(0, MAX_DISCOVERED) },
    ),
  clear: () => set({ discovered: [] }),
}));
