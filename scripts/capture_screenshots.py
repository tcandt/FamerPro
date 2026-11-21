import subprocess
import time
import os
from PIL import Image

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
PORT = 4173

views = [
    ("overview", "screenshots/v2_tab_01_overview.png"),
    ("devices", "screenshots/v2_tab_02_devices.png"),
    ("studio", "screenshots/v2_tab_03_studio.png"),
    ("tasks", "screenshots/v2_tab_04_tasks.png"),
    ("runs", "screenshots/v2_tab_05_runs.png"),
    ("accounts", "screenshots/v2_tab_06_accounts.png"),
    ("network", "screenshots/v2_tab_07_network.png"),
    ("gallery", "screenshots/v2_tab_08_gallery.png"),
    ("settings", "screenshots/v2_tab_09_settings.png"),
]

def main():
    print("Starting Python HTTP server on port", PORT)
    p = subprocess.Popen(['python', '-m', 'http.server', str(PORT), '--directory', 'apps/desktop/dist'])
    time.sleep(1)

    try:
        for view_name, rel_path in views:
            abs_path = os.path.abspath(rel_path)
            url = f"http://127.0.0.1:{PORT}/?view={view_name}"
            cmd = [
                CHROME,
                "--headless=new",
                "--disable-gpu",
                "--hide-scrollbars",
                "--window-size=1440,900",
                "--virtual-time-budget=2500",
                f"--screenshot={abs_path}",
                url,
            ]
            print(f"Capturing {view_name} -> {rel_path}...")
            subprocess.run(cmd, check=True)
            if os.path.exists(abs_path):
                im = Image.open(abs_path)
                print(f"  OK: {rel_path} size={im.size} bytes={os.path.getsize(abs_path):,}")
            else:
                print(f"  FAILED: {rel_path}")
    finally:
        print("Stopping HTTP server...")
        p.kill()

if __name__ == "__main__":
    main()
