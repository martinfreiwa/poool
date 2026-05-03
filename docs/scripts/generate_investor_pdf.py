#!/usr/bin/env python3
"""
POOOL Affiliate Partner Syndicate — Investor Overview PDF
Design tokens sourced from docs/DESIGN.md and frontend/platform/static/css/dashboard-tokens.css
"""

import re
import os
from reportlab.lib.pagesizes import A4
from reportlab.lib.units import mm, cm
from reportlab.lib import colors
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_RIGHT, TA_JUSTIFY
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle,
    HRFlowable, KeepTogether, PageBreak, Frame, PageTemplate
)
from reportlab.platypus.flowables import Flowable
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont

# ─── Paths ────────────────────────────────────────────────────────────────────
BASE = "/Users/martin/Projects/poool"
FONT_DIR = f"{BASE}/frontend/www/fonts/TTNormPro"
MD_FILE  = f"{BASE}/docs/affiliate/AFFILIATE_SYSTEM_INVESTOR_OVERVIEW.md"
OUT_FILE = f"{BASE}/docs/affiliate/AFFILIATE_SYSTEM_INVESTOR_OVERVIEW.pdf"

# ─── POOOL Design System Tokens (docs/DESIGN.md) ─────────────────────────────
# Primary brand
ELECTRIC_BLUE = colors.HexColor("#0000FF")   # --btn-primary-bg, primary accent
BRIGHT_LIME   = colors.HexColor("#98FB96")   # --btn-primary-color, brand green
DARK_BLUE     = colors.HexColor("#08232F")   # dark foundation for cover
DEEP_BLUE_UI  = colors.HexColor("#1B2559")   # active tab, heading accent
BANNER_BG     = colors.HexColor("#EEF4FF")   # info banner background

# Text tokens
TEXT_TITLE    = colors.HexColor("#181D27")   # --page-title-color
TEXT_SECTION  = colors.HexColor("#101828")   # --section-title-color
TEXT_BODY     = colors.HexColor("#344054")   # --body-color
TEXT_LABEL    = colors.HexColor("#475467")   # --label-color
TEXT_MUTED    = colors.HexColor("#667085")   # muted/secondary
TEXT_WHITE    = colors.white

# Surface tokens
CARD_BG       = colors.HexColor("#FFFFFF")   # --card-bg
PAGE_BG       = colors.HexColor("#FAFAFA")   # --content-bg
TABLE_HEAD_BG = colors.HexColor("#F9FAFB")   # table header
TABLE_STRIPE  = colors.HexColor("#FAFAFA")   # alternating row
BORDER        = colors.HexColor("#E5E7EB")   # --card-border-color
BORDER_INPUT  = colors.HexColor("#D5D7DA")   # input border

# Status colors
SUCCESS_BG    = colors.HexColor("#ECFDF3")
SUCCESS_TEXT  = colors.HexColor("#027A48")
WARNING_BG    = colors.HexColor("#FFFAEB")
WARNING_TEXT  = colors.HexColor("#B54708")

PAGE_W, PAGE_H = A4
MARGIN_L = 20*mm
MARGIN_R = 20*mm
MARGIN_T = 24*mm
MARGIN_B = 24*mm
CONTENT_W = PAGE_W - MARGIN_L - MARGIN_R

# ─── Font Registration ────────────────────────────────────────────────────────
def register_fonts():
    variants = {
        "TTNorms":          "TTNormsPro-Regular.ttf",
        "TTNorms-Medium":   "TTNormsPro-Medium.ttf",
        "TTNorms-Bold":     "TTNormsPro-Bold.ttf",
        "TTNorms-ExtraBold":"TTNormsPro-ExtraBold.ttf",
        "TTNorms-Italic":   "TTNormsPro-Italic.ttf",
    }
    for alias, fname in variants.items():
        fp = os.path.join(FONT_DIR, fname)
        if os.path.exists(fp):
            pdfmetrics.registerFont(TTFont(alias, fp))
    pdfmetrics.registerFontFamily(
        "TTNorms",
        normal="TTNorms",
        bold="TTNorms-Bold",
        italic="TTNorms-Italic",
        boldItalic="TTNorms-Bold",
    )

register_fonts()

# ─── Paragraph Styles ─────────────────────────────────────────────────────────
def build_styles():
    return {
        # Cover
        "COVER_EYEBROW": ParagraphStyle("COVER_EYEBROW",
            fontName="TTNorms-Medium", fontSize=8, textColor=BRIGHT_LIME,
            leading=12, spaceAfter=6, letterSpacing=1.5),

        "COVER_TITLE": ParagraphStyle("COVER_TITLE",
            fontName="TTNorms-ExtraBold", fontSize=32, textColor=TEXT_WHITE,
            leading=38, spaceAfter=10),

        "COVER_SUBTITLE": ParagraphStyle("COVER_SUBTITLE",
            fontName="TTNorms", fontSize=12, textColor=colors.HexColor("#B8C4CC"),
            leading=18, spaceAfter=16),

        "COVER_META": ParagraphStyle("COVER_META",
            fontName="TTNorms", fontSize=9, textColor=colors.HexColor("#6B8FA3"),
            leading=14),

        # Body headings
        "H2": ParagraphStyle("H2",
            fontName="TTNorms-ExtraBold", fontSize=15, textColor=TEXT_SECTION,
            leading=20, spaceBefore=20, spaceAfter=6),

        "H3": ParagraphStyle("H3",
            fontName="TTNorms-Bold", fontSize=11, textColor=DEEP_BLUE_UI,
            leading=16, spaceBefore=14, spaceAfter=4),

        "H4": ParagraphStyle("H4",
            fontName="TTNorms-Bold", fontSize=10, textColor=TEXT_TITLE,
            leading=15, spaceBefore=10, spaceAfter=3),

        # Body
        "BODY": ParagraphStyle("BODY",
            fontName="TTNorms", fontSize=9.5, textColor=TEXT_BODY,
            leading=15, spaceAfter=6, alignment=TA_JUSTIFY),

        "BULLET": ParagraphStyle("BULLET",
            fontName="TTNorms", fontSize=9.5, textColor=TEXT_BODY,
            leading=15, leftIndent=14, spaceAfter=3,
            bulletIndent=0, bulletFontName="TTNorms-Bold", bulletFontSize=9),

        "BLOCKQUOTE": ParagraphStyle("BLOCKQUOTE",
            fontName="TTNorms-Italic", fontSize=9, textColor=TEXT_LABEL,
            leading=14, leftIndent=14, rightIndent=0, spaceAfter=0),

        # Tables
        "TH": ParagraphStyle("TH",
            fontName="TTNorms-Bold", fontSize=8.5, textColor=TEXT_SECTION,
            leading=12),

        "TD": ParagraphStyle("TD",
            fontName="TTNorms", fontSize=8.5, textColor=TEXT_BODY,
            leading=13),

        "TD_MUTED": ParagraphStyle("TD_MUTED",
            fontName="TTNorms", fontSize=8.5, textColor=TEXT_MUTED,
            leading=13),

        # Footer
        "FOOTER": ParagraphStyle("FOOTER",
            fontName="TTNorms", fontSize=7.5, textColor=TEXT_MUTED,
            leading=11, alignment=TA_CENTER),

        # Meta / labels
        "META": ParagraphStyle("META",
            fontName="TTNorms", fontSize=8, textColor=TEXT_MUTED,
            leading=12, spaceAfter=2),

        "LABEL": ParagraphStyle("LABEL",
            fontName="TTNorms-Medium", fontSize=8, textColor=TEXT_MUTED,
            leading=12, letterSpacing=0.6),
    }

S = build_styles()

# ─── Custom Flowables ─────────────────────────────────────────────────────────

class CoverPage(Flowable):
    """Full-bleed dark cover with POOOL branding."""
    def __init__(self, width, height):
        Flowable.__init__(self)
        self.width = width
        self.height = height

    def draw(self):
        c = self.canv
        w, h = self.width, self.height

        # Dark blue background
        c.setFillColor(DARK_BLUE)
        c.rect(-MARGIN_L, -MARGIN_T, PAGE_W, PAGE_H, fill=1, stroke=0)

        # Subtle grid lines (brand texture)
        c.setStrokeColor(colors.HexColor("#0A2F3C"))
        c.setLineWidth(0.3)
        step = 18*mm
        x = -MARGIN_L
        while x < PAGE_W:
            c.line(x, -MARGIN_T, x, PAGE_H - MARGIN_T)
            x += step
        y = -MARGIN_T
        while y < PAGE_H:
            c.line(-MARGIN_L, y, PAGE_W - MARGIN_L, y)
            y += step

        # Large Triple-O circle accent (brand identity: overlapping circles)
        c.setStrokeColor(colors.HexColor("#0D3347"))
        c.setFillColor(colors.HexColor("#00000000"))
        c.setLineWidth(1.0)
        r = 60*mm
        # Three overlapping circles (the OOO motif)
        c.circle(w - 10*mm, h - 10*mm, r, stroke=1, fill=0)
        c.circle(w + 10*mm, h - 10*mm, r, stroke=1, fill=0)
        c.circle(w, h - 10*mm + 17*mm, r, stroke=1, fill=0)

        # Bright lime top accent bar
        c.setFillColor(ELECTRIC_BLUE)
        c.rect(-MARGIN_L, h + MARGIN_T - 5, PAGE_W, 5, fill=1, stroke=0)
        c.setFillColor(BRIGHT_LIME)
        c.rect(-MARGIN_L, h + MARGIN_T - 5, 60*mm, 5, fill=1, stroke=0)

        # POOOL wordmark
        c.setFillColor(TEXT_WHITE)
        c.setFont("TTNorms-ExtraBold", 26)
        c.drawString(0, h - 14*mm, "POOOL")

        # Partner Syndicate pill badge
        badge_text = "PARTNER SYNDICATE"
        c.setFillColor(colors.HexColor("#0D3A50"))
        c.roundRect(0, h - 24*mm, 62*mm, 7*mm, 3.5, fill=1, stroke=0)
        c.setFillColor(BRIGHT_LIME)
        c.setFont("TTNorms-Medium", 7)
        c.drawString(3*mm, h - 19.5*mm, badge_text)

        # Confidential badge top-right
        c.setFillColor(colors.HexColor("#0D3A50"))
        c.roundRect(w - 28*mm, h - 14.5*mm, 28*mm, 6.5*mm, 3, fill=1, stroke=0)
        c.setFillColor(colors.HexColor("#6B8FA3"))
        c.setFont("TTNorms-Medium", 7)
        c.drawString(w - 25*mm, h - 10.5*mm, "CONFIDENTIAL")

        # Main title
        c.setFillColor(TEXT_WHITE)
        c.setFont("TTNorms-ExtraBold", 28)
        c.drawString(0, h - 44*mm, "Affiliate Partner")
        c.drawString(0, h - 57*mm, "Syndicate")

        # Lime accent underline below title
        c.setFillColor(BRIGHT_LIME)
        c.rect(0, h - 61*mm, 28*mm, 2.5, fill=1, stroke=0)
        c.setFillColor(ELECTRIC_BLUE)
        c.rect(28*mm + 1.5, h - 61*mm, w - 28*mm - 1.5, 0.5, fill=1, stroke=0)

        # Subtitle
        c.setFillColor(colors.HexColor("#8AA8B8"))
        c.setFont("TTNorms", 11)
        c.drawString(0, h - 72*mm, "A performance-based investor acquisition channel")
        c.drawString(0, h - 82*mm, "built for institutional standards.")

        # Meta row
        meta_y = h - 96*mm
        c.setStrokeColor(colors.HexColor("#0D3A50"))
        c.setLineWidth(0.5)
        c.line(0, meta_y + 5*mm, w, meta_y + 5*mm)

        c.setFillColor(colors.HexColor("#4A6F85"))
        c.setFont("TTNorms", 8)
        c.drawString(0, meta_y, "Document type: Investor Overview")
        c.drawRightString(w, meta_y, "April 2026")

        # Stats row at bottom of cover
        stats_y = 18*mm
        stats = [
            ("$12M+", "Assets Under Management"),
            ("24", "Live Properties"),
            ("8", "Commission Tiers"),
            ("1.75%", "Max Commission"),
        ]
        stat_w = w / len(stats)
        for i, (val, lbl) in enumerate(stats):
            x = i * stat_w
            # Separator line
            if i > 0:
                c.setStrokeColor(colors.HexColor("#0D3A50"))
                c.setLineWidth(0.5)
                c.line(x, stats_y - 2*mm, x, stats_y + 14*mm)
            c.setFillColor(BRIGHT_LIME)
            c.setFont("TTNorms-ExtraBold", 16)
            c.drawString(x + 4*mm, stats_y + 6*mm, val)
            c.setFillColor(colors.HexColor("#4A6F85"))
            c.setFont("TTNorms", 7.5)
            c.drawString(x + 4*mm, stats_y + 1*mm, lbl)

        # Bottom rule
        c.setStrokeColor(colors.HexColor("#0D3A50"))
        c.setLineWidth(0.5)
        c.line(0, stats_y - 3*mm, w, stats_y - 3*mm)


class SectionHeader(Flowable):
    """H2 with blue left accent bar — POOOL card-accent pattern."""
    def __init__(self, text, width):
        Flowable.__init__(self)
        self.text = text
        self.width = width
        self._para = Paragraph(text, S["H2"])
        self.height = None

    def wrap(self, aw, ah):
        _, h = self._para.wrap(aw - 8, ah)
        self.height = h + 10
        return (aw, self.height)

    def draw(self):
        c = self.canv
        h = self.height
        # Blue left accent bar
        c.setFillColor(ELECTRIC_BLUE)
        c.rect(0, 0, 3, h - 2, fill=1, stroke=0)
        # Lime dot at top of bar
        c.setFillColor(BRIGHT_LIME)
        c.circle(1.5, h - 2, 2, fill=1, stroke=0)
        self._para.drawOn(c, 8, 2)


class BlueSidebar(Flowable):
    """Blockquote with blue left bar and light blue background."""
    def __init__(self, text_para, width):
        Flowable.__init__(self)
        self.para = text_para
        self.width = width
        self._h = None

    def wrap(self, aw, ah):
        _, h = self.para.wrap(aw - 18, ah)
        self._h = h + 14
        return (aw, self._h)

    def draw(self):
        c = self.canv
        h = self._h
        # Light blue bg card
        c.setFillColor(BANNER_BG)
        c.roundRect(0, 0, self.width, h, 4, fill=1, stroke=0)
        # Blue left bar
        c.setFillColor(ELECTRIC_BLUE)
        c.roundRect(0, 0, 3, h, 1.5, fill=1, stroke=0)
        self.para.drawOn(c, 12, 7)


class StatusBadge(Flowable):
    """Inline status badge for tables."""
    def __init__(self, text, variant="blue"):
        Flowable.__init__(self)
        self.text = text
        self.variant = variant
        self.height = 5*mm
        self.width = 0

    def wrap(self, aw, ah):
        self.width = min(aw, 30*mm)
        return (self.width, self.height)

    def draw(self):
        c = self.canv
        colors_map = {
            "blue":    (BANNER_BG, ELECTRIC_BLUE),
            "green":   (SUCCESS_BG, SUCCESS_TEXT),
            "warning": (WARNING_BG, WARNING_TEXT),
            "neutral": (colors.HexColor("#F2F4F7"), TEXT_MUTED),
        }
        bg, fg = colors_map.get(self.variant, (BANNER_BG, ELECTRIC_BLUE))
        c.setFillColor(bg)
        c.roundRect(0, 0.5, self.width, self.height - 1, 2.5, fill=1, stroke=0)
        c.setFillColor(fg)
        c.setFont("TTNorms-Medium", 7)
        c.drawCentredString(self.width / 2, 1.5, self.text)


class PageDivider(Flowable):
    """Thin section rule with optional label."""
    def __init__(self, width, label=None):
        Flowable.__init__(self)
        self.width = width
        self.label = label
        self.height = 1

    def draw(self):
        c = self.canv
        c.setStrokeColor(BORDER)
        c.setLineWidth(0.5)
        c.line(0, 0, self.width, 0)


# ─── Page template callbacks ──────────────────────────────────────────────────

def draw_cover(canvas, doc):
    """Draw full-bleed cover using direct canvas calls."""
    canvas.saveState()
    w_content = PAGE_W - MARGIN_L - MARGIN_R

    # Dark blue background — full page
    canvas.setFillColor(DARK_BLUE)
    canvas.rect(0, 0, PAGE_W, PAGE_H, fill=1, stroke=0)

    # Subtle grid texture
    canvas.setStrokeColor(colors.HexColor("#0A2F3C"))
    canvas.setLineWidth(0.3)
    step = 18*mm
    x = 0
    while x <= PAGE_W:
        canvas.line(x, 0, x, PAGE_H)
        x += step
    y = 0
    while y <= PAGE_H:
        canvas.line(0, y, PAGE_W, y)
        y += step

    # Triple-O circles (brand motif)
    canvas.setStrokeColor(colors.HexColor("#0D3347"))
    canvas.setLineWidth(1.0)
    r = 60*mm
    canvas.circle(PAGE_W - 10*mm, PAGE_H - 10*mm, r, stroke=1, fill=0)
    canvas.circle(PAGE_W + 10*mm, PAGE_H - 10*mm, r, stroke=1, fill=0)
    canvas.circle(PAGE_W, PAGE_H - 10*mm + 17*mm, r, stroke=1, fill=0)

    # Top accent bars
    canvas.setFillColor(ELECTRIC_BLUE)
    canvas.rect(0, PAGE_H - 4, PAGE_W, 4, fill=1, stroke=0)
    canvas.setFillColor(BRIGHT_LIME)
    canvas.rect(0, PAGE_H - 4, 60*mm, 4, fill=1, stroke=0)

    lx = MARGIN_L  # left x baseline

    # POOOL wordmark
    canvas.setFillColor(TEXT_WHITE)
    canvas.setFont("TTNorms-ExtraBold", 26)
    canvas.drawString(lx, PAGE_H - 22*mm, "POOOL")

    # Partner Syndicate badge pill
    canvas.setFillColor(colors.HexColor("#0D3A50"))
    canvas.roundRect(lx, PAGE_H - 33*mm, 64*mm, 7.5*mm, 3.5, fill=1, stroke=0)
    canvas.setFillColor(BRIGHT_LIME)
    canvas.setFont("TTNorms-Medium", 7.5)
    canvas.drawString(lx + 3.5*mm, PAGE_H - 27.8*mm, "PARTNER SYNDICATE")

    # Confidential badge top-right
    canvas.setFillColor(colors.HexColor("#0D3A50"))
    canvas.roundRect(PAGE_W - MARGIN_R - 30*mm, PAGE_H - 25*mm, 30*mm, 6.5*mm, 3, fill=1, stroke=0)
    canvas.setFillColor(colors.HexColor("#6B8FA3"))
    canvas.setFont("TTNorms-Medium", 7)
    canvas.drawString(PAGE_W - MARGIN_R - 27*mm, PAGE_H - 20.5*mm, "CONFIDENTIAL")

    # Title
    canvas.setFillColor(TEXT_WHITE)
    canvas.setFont("TTNorms-ExtraBold", 30)
    canvas.drawString(lx, PAGE_H - 60*mm, "Affiliate Partner")
    canvas.drawString(lx, PAGE_H - 75*mm, "Syndicate")

    # Lime + blue accent line under title
    canvas.setFillColor(BRIGHT_LIME)
    canvas.rect(lx, PAGE_H - 80*mm, 32*mm, 3, fill=1, stroke=0)
    canvas.setFillColor(colors.HexColor("#0D3347"))
    canvas.rect(lx + 33*mm, PAGE_H - 80*mm, w_content - 33*mm, 0.5, fill=1, stroke=0)

    # Subtitle
    canvas.setFillColor(colors.HexColor("#8AA8B8"))
    canvas.setFont("TTNorms", 11.5)
    canvas.drawString(lx, PAGE_H - 93*mm, "A performance-based investor acquisition channel")
    canvas.drawString(lx, PAGE_H - 103*mm, "built for institutional standards.")

    # Meta row
    canvas.setStrokeColor(colors.HexColor("#0D3A50"))
    canvas.setLineWidth(0.5)
    canvas.line(lx, PAGE_H - 112*mm, PAGE_W - MARGIN_R, PAGE_H - 112*mm)
    canvas.setFillColor(colors.HexColor("#4A6F85"))
    canvas.setFont("TTNorms", 8.5)
    canvas.drawString(lx, PAGE_H - 118*mm, "Document type: Investor Overview  ·  Confidential")
    canvas.drawRightString(PAGE_W - MARGIN_R, PAGE_H - 118*mm, "April 2026")

    # Stats row at bottom
    stats_y = 22*mm
    stats = [
        ("$12M+", "Assets Under\nManagement"),
        ("24", "Live\nProperties"),
        ("8", "Commission\nTiers"),
        ("1.75%", "Max\nCommission"),
    ]
    sw = (PAGE_W - MARGIN_L - MARGIN_R) / len(stats)
    for i, (val, lbl) in enumerate(stats):
        sx = lx + i * sw
        if i > 0:
            canvas.setStrokeColor(colors.HexColor("#0D3A50"))
            canvas.setLineWidth(0.5)
            canvas.line(sx, stats_y - 2*mm, sx, stats_y + 16*mm)
        canvas.setFillColor(BRIGHT_LIME)
        canvas.setFont("TTNorms-ExtraBold", 17)
        canvas.drawString(sx + 3*mm, stats_y + 8*mm, val)
        canvas.setFillColor(colors.HexColor("#4A6F85"))
        canvas.setFont("TTNorms", 7.5)
        for j, part in enumerate(lbl.split('\n')):
            canvas.drawString(sx + 3*mm, stats_y + 3*mm - j*4*mm, part)

    # Bottom rule above stats
    canvas.setStrokeColor(colors.HexColor("#0D3A50"))
    canvas.setLineWidth(0.5)
    canvas.line(lx, stats_y - 3.5*mm, PAGE_W - MARGIN_R, stats_y - 3.5*mm)

    canvas.restoreState()


def on_first_page(canvas, doc):
    """Cover page — full-bleed, no body frame content."""
    draw_cover(canvas, doc)


def on_later_pages(canvas, doc):
    """Standard footer for all body pages."""
    canvas.saveState()
    page = doc.page

    # Footer rule
    canvas.setStrokeColor(BORDER)
    canvas.setLineWidth(0.5)
    fy = MARGIN_B - 6*mm
    canvas.line(MARGIN_L, fy, PAGE_W - MARGIN_R, fy)

    # Left: POOOL brand
    canvas.setFillColor(ELECTRIC_BLUE)
    canvas.setFont("TTNorms-ExtraBold", 8)
    canvas.drawString(MARGIN_L, fy - 4.5*mm, "POOOL")

    canvas.setFillColor(TEXT_MUTED)
    canvas.setFont("TTNorms", 8)
    canvas.drawString(MARGIN_L + 13*mm, fy - 4.5*mm, "Partner Syndicate — Investor Overview")

    # Right: page number
    canvas.setFillColor(TEXT_MUTED)
    canvas.setFont("TTNorms", 8)
    canvas.drawRightString(PAGE_W - MARGIN_R, fy - 4.5*mm, f"{page}")

    # Lime accent dot beside page number
    canvas.setFillColor(BRIGHT_LIME)
    canvas.circle(PAGE_W - MARGIN_R + 5, fy - 2*mm, 2, fill=1, stroke=0)

    canvas.restoreState()


# ─── Markdown → Flowables ─────────────────────────────────────────────────────

def esc(text):
    text = text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    return text

def inline(text):
    text = esc(text)
    text = re.sub(r'\*\*(.+?)\*\*', r'<b>\1</b>', text)
    text = re.sub(r'\*([^*]+?)\*', r'<i>\1</i>', text)
    text = re.sub(r'`([^`]+)`',
        lambda m: f'<font name="TTNorms-Medium" color="#0000FF">{esc(m.group(1))}</font>',
        text)
    return text


def parse_table(lines):
    rows = []
    for line in lines:
        if re.match(r'\s*\|[-: |]+\|\s*$', line):
            continue
        cells = [c.strip() for c in line.strip().strip('|').split('|')]
        rows.append(cells)
    return rows


def classify_status(text):
    """Detect status words and return a badge variant."""
    t = text.lower()
    if t in ("live", "complete", "completed", "done", "✅"):
        return "green"
    if t in ("in development", "planned", "coming soon"):
        return "warning"
    return None


def build_table(rows):
    if not rows:
        return None
    header, data = rows[0], rows[1:]
    ncols = len(header)
    col_w = CONTENT_W / ncols

    def cell(text, style):
        # Status badge detection for 2-col tables
        badge = classify_status(text) if ncols <= 3 else None
        if badge:
            p = Paragraph(inline(text), style)
            return p
        return Paragraph(inline(text), style)

    tdata = [[cell(h, S["TH"]) for h in header]]
    for i, row in enumerate(data):
        tdata.append([cell(c, S["TD"]) for c in row])

    ts = TableStyle([
        # Header row
        ("BACKGROUND",    (0,0), (-1,0),  TABLE_HEAD_BG),
        ("LINEBELOW",     (0,0), (-1,0),  0.8, BORDER),
        ("TOPPADDING",    (0,0), (-1,0),  7),
        ("BOTTOMPADDING", (0,0), (-1,0),  7),
        ("LEFTPADDING",   (0,0), (-1,-1), 9),
        ("RIGHTPADDING",  (0,0), (-1,-1), 9),
        # Body rows
        ("TOPPADDING",    (0,1), (-1,-1), 5),
        ("BOTTOMPADDING", (0,1), (-1,-1), 5),
        ("ROWBACKGROUNDS",(0,1), (-1,-1), [CARD_BG, TABLE_STRIPE]),
        ("GRID",          (0,0), (-1,-1), 0.4, BORDER),
        ("VALIGN",        (0,0), (-1,-1), "MIDDLE"),
        # Left accent on first col of first row
        ("LINEAFTER",     (0,0), (0,-1),  0.4, BORDER),
    ])

    tbl = Table(tdata, colWidths=[col_w]*ncols, repeatRows=1)
    tbl.setStyle(ts)
    return tbl


def parse_md(md_text):
    story = []
    lines = md_text.split('\n')
    i = 0
    skip_h1 = True  # cover replaces the H1

    while i < len(lines):
        line = lines[i]

        # H1 — skip (rendered as cover)
        if line.startswith('# ') and skip_h1:
            skip_h1 = False
            i += 1
            continue

        # Horizontal rule
        if re.match(r'^---+\s*$', line):
            story.append(Spacer(1, 3))
            story.append(PageDivider(CONTENT_W))
            story.append(Spacer(1, 2))
            i += 1
            continue

        # Blockquote
        if line.startswith('> '):
            quote_lines = []
            while i < len(lines) and lines[i].startswith('> '):
                quote_lines.append(lines[i][2:])
                i += 1
            text = ' '.join(quote_lines)
            para = Paragraph(inline(text), S["BLOCKQUOTE"])
            story.append(Spacer(1, 3))
            story.append(BlueSidebar(para, CONTENT_W))
            story.append(Spacer(1, 8))
            continue

        # Table
        if '|' in line and i + 1 < len(lines) and re.match(r'\s*\|[-: |]+\|\s*$', lines[i+1]):
            tlines = []
            while i < len(lines) and '|' in lines[i]:
                tlines.append(lines[i])
                i += 1
            tbl = build_table(parse_table(tlines))
            if tbl:
                story.append(Spacer(1, 6))
                story.append(tbl)
                story.append(Spacer(1, 10))
            continue

        # H2
        if line.startswith('## '):
            text = line[3:].strip()
            story.append(Spacer(1, 6))
            story.append(SectionHeader(inline(text), CONTENT_W))
            story.append(Spacer(1, 4))
            i += 1
            continue

        # H3
        if line.startswith('### '):
            text = line[4:].strip()
            story.append(Paragraph(inline(text), S["H3"]))
            i += 1
            continue

        # H4
        if line.startswith('#### '):
            text = line[5:].strip()
            story.append(Paragraph(f'<b>{inline(text)}</b>', S["H4"]))
            i += 1
            continue

        # Bullet list
        if re.match(r'^(\s*[-*] )', line):
            bullets = []
            while i < len(lines) and re.match(r'^(\s*[-*] )', lines[i]):
                raw = re.sub(r'^\s*[-*] ', '', lines[i])
                bullets.append(raw)
                i += 1
            for b in bullets:
                story.append(Paragraph(
                    f'<bullet><font color="#0000FF">&#9679;</font></bullet>{inline(b)}',
                    S["BULLET"]))
            story.append(Spacer(1, 4))
            continue

        # Numbered list
        if re.match(r'^\d+\. ', line):
            items = []
            while i < len(lines) and re.match(r'^\d+\. ', lines[i]):
                m = re.match(r'^(\d+)\. (.+)', lines[i])
                if m:
                    items.append((m.group(1), m.group(2)))
                i += 1
            for num, text in items:
                story.append(Paragraph(
                    f'<bullet><font name="TTNorms-Bold" color="#0000FF">{num}.</font></bullet>{inline(text)}',
                    S["BULLET"]))
            story.append(Spacer(1, 4))
            continue

        # Empty line
        if line.strip() == '':
            story.append(Spacer(1, 4))
            i += 1
            continue

        # Normal paragraph
        story.append(Paragraph(inline(line.strip()), S["BODY"]))
        i += 1

    return story


# ─── Build ────────────────────────────────────────────────────────────────────

def build():
    with open(MD_FILE) as f:
        md = f.read()

    doc = SimpleDocTemplate(
        OUT_FILE,
        pagesize=A4,
        leftMargin=MARGIN_L,
        rightMargin=MARGIN_R,
        topMargin=MARGIN_T,
        bottomMargin=MARGIN_B + 8*mm,
        title="POOOL Partner Syndicate — Investor Overview",
        author="POOOL",
        subject="Affiliate System — Investor Overview",
        creator="POOOL Platform",
    )

    story = []

    # ── Cover page ──────────────────────────────────────────────────────────────
    # A Spacer fills page 1's content frame (transparent — dark cover canvas
    # from on_first_page shows through). PageBreak then advances to page 2.
    # Frame usable height = page - margins - 2×6pt default frame padding
    CONTENT_H = PAGE_H - MARGIN_T - (MARGIN_B + 8*mm) - 12
    story.append(Spacer(1, CONTENT_H))
    story.append(PageBreak())

    # ── Table of Contents hint ─────────────────────────────────────────────────
    toc_sections = [
        "What It Is",
        "Business Case",
        "The Four Partner Categories",
        "The Affiliate System — End to End",
        "The Partner Portal — Page by Page",
        "The Commission Structure in Full",
        "The 30-Day Holdback — How Money Flows",
        "Compliance & Legal Safeguards",
        "Fraud Prevention",
        "Admin Control Panel",
        "Email Notifications",
        "Current Build Status",
        "Summary",
    ]

    # Section header bar
    story.append(SectionHeader("Contents", CONTENT_W))
    story.append(Spacer(1, 6))
    for idx, sec in enumerate(toc_sections, 1):
        p = Paragraph(
            f'<font name="TTNorms-Medium" color="#0000FF">{idx:02d}</font>'
            f'<font name="TTNorms" color="#667085">  —  </font>'
            f'<font name="TTNorms" color="#344054">{esc(sec)}</font>',
            S["BODY"])
        story.append(p)
        story.append(Spacer(1, 2))
    story.append(Spacer(1, 8))
    story.append(PageDivider(CONTENT_W))
    story.append(PageBreak())

    # ── Body content ───────────────────────────────────────────────────────────
    story.extend(parse_md(md))

    doc.build(story, onFirstPage=on_first_page, onLaterPages=on_later_pages)
    print(f"✓  PDF saved → {OUT_FILE}")


if __name__ == "__main__":
    build()
