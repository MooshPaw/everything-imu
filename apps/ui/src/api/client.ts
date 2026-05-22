import { commands, events } from "./bindings";

export const api = commands;

// MAC address bytes as the Tauri commands expect them — a fixed-length
// tuple. Re-exported here so call sites don't have to import the
// auto-generated binding shape directly.
export type Mac = [number, number, number, number, number, number];

export type {
  BiasUpdate,
  ConnectionStatusUpdate,
  DeviceMetadataDto,
  FusionAlgoDto,
  HapticConfigDto,
  HapticModeDto,
  HapticRuleDto,
  ImuSampleUpdate,
  LatencyEntry,
  LatencyUpdate,
  LogEntryDto,
  MagCalibrationDto,
  MagCalProgressDto,
  MountingOrientationDto,
  PerDeviceSettingsDto,
  SettingsDto,
  TrackerSnapshot,
} from "./bindings";
export { events };
