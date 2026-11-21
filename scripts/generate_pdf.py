import subprocess
import os
import pymupdf

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
HTML = os.path.abspath("FARMERPRO_UI_GALLERY_REPORT.html")
PDF = os.path.abspath("FARMERPRO_UI_GALLERY_REPORT.pdf")

cmd = [
    CHROME,
    "--headless=new",
    "--disable-gpu",
    "--no-pdf-header-footer",
    f"--print-to-pdf={PDF}",
    f"file:///{HTML.replace(os.sep, '/')}"
]

print("Running Chrome print-to-pdf...")
subprocess.run(cmd, check=True)
print(f"Generated PDF: {PDF} ({os.path.getsize(PDF):,} bytes)")

doc = pymupdf.open(PDF)
print(f"Total pages: {len(doc)}")
for i, page in enumerate(doc):
    text = [l.strip() for l in page.get_text().splitlines() if l.strip()]
    first_line = text[0] if text else "EMPTY"
    images = page.get_images()
    print(f"Page {i+1:02d}: images={len(images)}, title='{first_line[:65]}'")
