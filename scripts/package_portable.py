import os
import shutil
import zipfile

def package():
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    target_exe = os.path.join(repo_root, "target", "release", "farmerpro.exe")
    dist_dir = os.path.join(repo_root, "dist-portable", "FarmerPro")
    dest_exe = os.path.join(dist_dir, "FarmerPro.exe")
    zip_path = os.path.join(repo_root, "FarmerPro-v0.1.0-win-x64-portable.zip")

    print(f"Copying {target_exe} -> {dest_exe}...")
    shutil.copy2(target_exe, dest_exe)

    if os.path.exists(zip_path):
        os.remove(zip_path)

    print(f"Creating {zip_path} with standard forward slash '/' separators...")
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for root, dirs, files in os.walk(dist_dir):
            for file in files:
                full_path = os.path.join(root, file)
                rel_path = os.path.relpath(full_path, os.path.dirname(dist_dir))
                # Ensure forward slashes strictly
                arc_name = rel_path.replace("\\", "/")
                zf.write(full_path, arc_name)
                print(f"  + {arc_name} ({os.path.getsize(full_path):,} bytes)")

    print(f"Done! Zip file size: {os.path.getsize(zip_path):,} bytes")

if __name__ == "__main__":
    package()
