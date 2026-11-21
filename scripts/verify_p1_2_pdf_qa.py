import os
import re
import json
import hashlib
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
PDF_MAIN = os.path.join(ROOT, "FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf")
PDF_COPY = os.path.join(ROOT, "artifacts", "p1_2_evidence", "FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf")
RENDER_DIR = os.path.join(ROOT, "artifacts", "p1_2_evidence", "pdf_rendered_pages")
QA_JSON = os.path.join(ROOT, "artifacts", "p1_2_evidence", "pdf_quality_assurance_report.json")
os.makedirs(RENDER_DIR, exist_ok=True)

# Clean old rendered PNGs first
for old_f in os.listdir(RENDER_DIR):
    if old_f.endswith(".png"):
        os.remove(os.path.join(RENDER_DIR, old_f))

def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()

sha_main = sha256_file(PDF_MAIN)
sha_copy = sha256_file(PDF_COPY)
assert sha_main == sha_copy, "Main PDF and Evidence Copy SHA-256 mismatch!"

doc = pymupdf.open(PDF_MAIN)
total_pages = len(doc)
toc_bookmarks = doc.get_toc()
toc_page_links = len(doc[1].get_links()) + len(doc[2].get_links())

page_reports = []
total_embedded_images = 0
aspect_ratio_errors = []
clipping_warnings = []
encoding_errors = []
small_font_warnings = []
global_min_body_pt = 99.0

for idx, page in enumerate(doc):
    pno = idx + 1
    pix = page.get_pixmap(dpi=130)
    png_path = os.path.join(RENDER_DIR, f"page_{pno:02d}.png")
    pix.save(png_path)

    text = page.get_text("text")
    if "\ufffd" in text:
        encoding_errors.append(f"Page {pno:02d}: Contains replacement char U+FFFD")
    stray = re.findall(r"[\u0400-\u04FF\u4E00-\u9FFF]+", text)
    if stray:
        encoding_errors.append(f"Page {pno:02d}: Contains stray CJK/Cyrillic {stray}")

    expected_footer = f"Trang {pno:02d} / {total_pages:02d}"
    footer_ok = expected_footer in text

    # Check font sizes inside main content (ignore header/footer/badge/caption small tags)
    pdict = page.get_text("dict")
    page_min_pt = 99.0
    for blk in pdict.get("blocks", []):
        if blk.get("type") != 0:
            continue
        bbox = blk.get("bbox", [0, 0, 0, 0])
        if bbox[1] < 36 or bbox[1] > 558:
            continue
        for ln in blk.get("lines", []):
            for sp in ln.get("spans", []):
                stxt = sp.get("text", "").strip()
                sz = sp.get("size", 0.0)
                if len(stxt) >= 3 and sz > 0:
                    page_min_pt = min(page_min_pt, sz)
                    global_min_body_pt = min(global_min_body_pt, sz)
                    if sz < 7.2:
                        small_font_warnings.append(f"Page {pno:02d}: {sz:.2f}pt on '{stxt[:40]}'")

    blocks = page.get_text("blocks")
    max_content_y1 = 0.0
    footer_y0 = 595.0
    for b in blocks:
        x0, y0, x1, y1, btxt, bno, btype = b
        btxt_s = btxt.strip()
        if "Hồ sơ nghiệm thu Kiến trúc" in btxt_s or expected_footer in btxt_s:
            footer_y0 = min(footer_y0, y0)
        else:
            max_content_y1 = max(max_content_y1, y1)
        if x1 > page.rect.width + 2 or y1 > page.rect.height + 2:
            clipping_warnings.append(f"Page {pno:02d}: Block exceeds page bounds ({x1:.1f}, {y1:.1f})")

    if max_content_y1 > footer_y0 + 1.5:
        clipping_warnings.append(
            f"Page {pno:02d}: Content bottom y1={max_content_y1:.1f}pt overlaps footer top y0={footer_y0:.1f}pt"
        )

    img_list = page.get_images(full=True)
    total_embedded_images += len(img_list)
    for im in img_list:
        xref = im[0]
        pix_w, pix_h = im[2], im[3]
        rects = page.get_image_rects(xref)
        for r in rects:
            if r.width > 0 and r.height > 0 and pix_w > 0 and pix_h > 0:
                src_ratio = pix_w / pix_h
                dst_ratio = r.width / r.height
                diff_pct = abs(src_ratio - dst_ratio) / src_ratio * 100.0
                if diff_pct > 1.5:
                    aspect_ratio_errors.append(
                        f"Page {pno:02d} xref={xref}: Aspect ratio distortion {diff_pct:.2f}% (src={src_ratio:.3f}, dst={dst_ratio:.3f})"
                    )

    page_reports.append({
        "page": pno,
        "dimensionsPt": f"{page.rect.width:.0f}x{page.rect.height:.0f}",
        "orientation": "A4 Landscape" if page.rect.width > page.rect.height else "Portrait",
        "embeddedImages": len(img_list),
        "footerVerified": footer_ok,
        "minContentFontPt": round(page_min_pt, 2) if page_min_pt < 90 else None,
        "maxContentBottomPt": round(max_content_y1, 1),
        "footerTopPt": round(footer_y0, 1),
        "clearancePt": round(footer_y0 - max_content_y1, 1),
        "renderedPng": png_path,
    })

full_doc_text = "\n".join(p.get_text("text") for p in doc)
doc.close()

required_keywords = [
    "v1.2.1-REVIEW",
    "PHẦN 1", "PHẦN 2", "PHẦN 3", "PHẦN 4", "PHẦN 5", "PHẦN 6", "PHẦN 7", "PHẦN 8", "PHẦN 9",
    "IMPLEMENTED", "PARTIAL", "MOCK", "MISSING", "PLANNED",
    "Ảnh 01", "Ảnh 02", "Ảnh 03A", "Ảnh 03B", "Ảnh 04", "Ảnh 05", "Ảnh 06", "Ảnh 07A", "Ảnh 07B", "Ảnh 08",
    "Sơ đồ 1.1", "Sơ đồ 1.2", "Sơ đồ 2.1",
    "Sơ đồ IV.1", "Sơ đồ IV.2", "Sơ đồ IV.3", "Sơ đồ IV.4", "Sơ đồ IV.5",
    "Sơ đồ IV.6", "Sơ đồ IV.7", "Sơ đồ IV.8", "Sơ đồ IV.9", "Sơ đồ IV.10",
    "ACTUAL IMPLEMENTATION", "PROPOSED DESIGN — NOT IMPLEMENTED",
    "RT_RCDATA", "7777", "Authenticode", "34/34", "1280 × 720", "1600 × 900", "1920 × 1080",
    "TC-01", "TC-14", "QD-01", "QD-07", "EVIDENCE_MANIFEST_SHA256"
]
missing_keywords = [kw for kw in required_keywords if kw not in full_doc_text]

qa_summary = {
    "reportVersion": "v1.2.1-REVIEW",
    "pdfPathMain": PDF_MAIN,
    "pdfPathEvidenceCopy": PDF_COPY,
    "fileSizeBytes": os.path.getsize(PDF_MAIN),
    "fileSizeMB": round(os.path.getsize(PDF_MAIN) / (1024 * 1024), 2),
    "sha256": sha_main,
    "sha256MatchesCopy": sha_main == sha_copy,
    "totalPages": total_pages,
    "pdfBookmarksCount": len(toc_bookmarks),
    "tocHyperlinkAnnotations": toc_page_links,
    "totalEmbeddedImageInstances": total_embedded_images,
    "allPagesA4Landscape": all(p["orientation"] == "A4 Landscape" for p in page_reports),
    "allFootersSequential": all(p["footerVerified"] for p in page_reports),
    "globalMinContentFontPt": round(global_min_body_pt, 2),
    "smallFontWarningsCount": len(small_font_warnings),
    "smallFontWarnings": small_font_warnings[:20],
    "encodingErrors": encoding_errors,
    "aspectRatioErrors": aspect_ratio_errors,
    "clippingOrOverlapWarnings": clipping_warnings,
    "missingRequiredKeywords": missing_keywords,
    "pages": page_reports,
}

with open(QA_JSON, "w", encoding="utf-8") as f:
    json.dump(qa_summary, f, indent=2, ensure_ascii=False)

print("=== PDF QA SUMMARY (v1.2.1-REVIEW) ===")
print(f"Total Pages               : {total_pages}")
print(f"PDF Bookmarks (Outline)   : {len(toc_bookmarks)}")
print(f"TOC Link Annotations      : {toc_page_links}")
print(f"Total Embedded Images     : {total_embedded_images}")
print(f"All Pages A4 Landscape    : {qa_summary['allPagesA4Landscape']}")
print(f"All Page Footers Verified : {qa_summary['allFootersSequential']}")
print(f"Min Content Font (pt)     : {qa_summary['globalMinContentFontPt']}pt (warnings <7.2pt: {len(small_font_warnings)})")
print(f"Encoding Errors           : {len(encoding_errors)} {encoding_errors}")
print(f"Aspect Ratio Distortions  : {len(aspect_ratio_errors)} {aspect_ratio_errors}")
print(f"Clipping/Overlap Warnings : {len(clipping_warnings)} {clipping_warnings}")
print(f"Missing Required Keywords : {len(missing_keywords)} {missing_keywords}")
for p in page_reports:
    print(f"  Page {p['page']:02d}: imgs={p['embeddedImages']:2d} | minPt={str(p['minContentFontPt']):>4} | maxY={p['maxContentBottomPt']:5.1f}pt | clear={p['clearancePt']:+5.1f}pt")
