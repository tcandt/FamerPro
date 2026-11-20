import { create } from 'zustand';
import { DeviceSummary, DeviceDetail } from '../types';

interface DeviceState {
  devices: DeviceSummary[];
  selectedDeviceId: string | null;
  selectedDeviceDetail: DeviceDetail | null;
  capturedScreenshot: string | null;
  isLoadingDevices: boolean;
  isInspecting: boolean;

  setDevices: (devices: DeviceSummary[]) => void;
  selectDevice: (id: string | null) => void;
  setSelectedDeviceDetail: (detail: DeviceDetail | null) => void;
  setCapturedScreenshot: (dataUrl: string | null) => void;
  setIsLoadingDevices: (loading: boolean) => void;
  setIsInspecting: (inspecting: boolean) => void;
}

const mockDevices: DeviceSummary[] = [
  {
    id: 'emulator-5554',
    adb_serial: 'emulator-5554',
    alias: 'Pixel 6 — Máy 01 (Demo)',
    android_api: 33,
    model: 'Pixel_6',
    width_px: 1080,
    height_px: 2400,
    rotation_deg: 0,
    connection_state: 'ready',
    updated_at: new Date().toISOString(),
  },
  {
    id: '192.168.1.105:5555',
    adb_serial: '192.168.1.105:5555',
    alias: 'Galaxy S22 — Máy 02 (Demo)',
    android_api: 31,
    model: 'SM-S901B',
    width_px: 1080,
    height_px: 2340,
    rotation_deg: 0,
    connection_state: 'ready',
    updated_at: new Date().toISOString(),
  },
  {
    id: '192.168.1.112:5555',
    adb_serial: '192.168.1.112:5555',
    alias: 'Xiaomi 12 — Máy 03 (Demo)',
    android_api: 29,
    model: '2201123G',
    width_px: 1080,
    height_px: 2400,
    rotation_deg: 0,
    connection_state: 'unauthorized',
    updated_at: new Date().toISOString(),
  },
];

export const useDeviceStore = create<DeviceState>((set) => ({
  devices: mockDevices,
  selectedDeviceId: 'emulator-5554',
  selectedDeviceDetail: {
    summary: mockDevices[0],
    battery_level: 95,
    is_screen_on: true,
    ip_address: '127.0.0.1',
    installed_packages: ['com.example.game', 'com.android.chrome'],
  },
  capturedScreenshot: null,
  isLoadingDevices: false,
  isInspecting: false,

  setDevices: (devices) => set({ devices }),
  selectDevice: (id) => set({ selectedDeviceId: id }),
  setSelectedDeviceDetail: (detail) => set({ selectedDeviceDetail: detail }),
  setCapturedScreenshot: (dataUrl) => set({ capturedScreenshot: dataUrl }),
  setIsLoadingDevices: (loading) => set({ isLoadingDevices: loading }),
  setIsInspecting: (inspecting) => set({ isInspecting: inspecting }),
}));
