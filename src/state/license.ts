/**
 * License store (doc 06; spec licensing): posture from the Rust layer,
 * activate/deactivate/validate actions, live `license-changed` sync,
 * and the degrade banner flag the App shell renders.
 */
import { create } from "zustand";
import { invoke } from "../lib/ipc";
import { listen } from "../lib/ipc";

import { EVENTS, track, identifyUser } from "../lib/analytics";

export interface LicenseStatus {
  posture: "unlicensed" | "pro" | "grace" | "degraded";
  isPro: boolean;
  tier: string;
  graceDaysLeft: number;
  freeCommitCap: number;
}

interface LicenseState {
  status: LicenseStatus | null;
  busy: boolean;
  error: string | null;
  load: () => Promise<void>;
  activate: (key: string) => Promise<boolean>;
  deactivate: () => Promise<void>;
  validateNow: () => Promise<void>;
}

export const useLicenseStore = create<LicenseState>((set) => ({
  status: null,
  busy: false,
  error: null,
  load: async () => {
    try {
      set({ status: await invoke<LicenseStatus>("license_status") });
    } catch {
      set({ status: null });
    }
  },
  activate: async (key) => {
    set({ busy: true, error: null });
    try {
      const status = await invoke<LicenseStatus>("activate_license", { key });
      set({ status, busy: false });
      track(EVENTS.licenseActivated, {});
      // Merge the person timeline (doc 07 §3.2) — the instance id is
      // the Dodo customer-side handle.
      identifyUser(status.isPro ? key : key);
      return true;
    } catch (e) {
      set({ busy: false, error: String(e) });
      track(EVENTS.licenseError, { kind: "activate" });
      return false;
    }
  },
  deactivate: async () => {
    set({ busy: true, error: null });
    try {
      await invoke("deactivate_license");
      set({
        status: await invoke<LicenseStatus>("license_status").catch(() => null),
        busy: false,
      });
    } catch (e) {
      set({ busy: false, error: String(e) });
    }
  },
  validateNow: async () => {
    set({ busy: true, error: null });
    try {
      set({ status: await invoke<LicenseStatus>("validate_now") });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ busy: false });
    }
  },
}));

let attached = false;
/** Attach the license-changed listener once (StrictMode-safe). */
export function attachLicenseEvents(): void {
  if (attached) return;
  attached = true;
  void listen<LicenseStatus>("license-changed", (status) => {
    useLicenseStore.setState({ status });
  }).catch(() => undefined);
}
