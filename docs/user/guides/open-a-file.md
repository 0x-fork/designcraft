---
title: Open a DesignCraft or InDesign (IDML) file
description: Open a .designcraft document or an InDesign Markup (.idml) file, and what to do about missing fonts and warnings.
order: 20
verified: 0.6.0
---

Choose **File**, then **Open…** (Ctrl+O), and pick a `.designcraft` or `.idml` file. The file opens as a new document.

## Steps

1. Open the **File** menu and choose **Open…**. On the start screen you can click **Open** instead.
2. In the file dialog, pick a file. The dialog lists two kinds: **DesignCraft** (`.designcraft`) and **InDesign Markup (IDML)** (`.idml`).
3. Confirm the dialog. The document opens as a new document.

## What the Open dialog accepts

The Open dialog lists `.designcraft` and `.idml` files only. To bring an image, a PDF or a text file into a document, use [Place](place-images-and-text.md) instead.

IDML is InDesign Markup, the exchange format InDesign can export. Linked images are read from next to the file or from its `Links` folder. If a `Document Fonts` folder sits beside the file, DesignCraft loads those fonts first.

## Missing fonts

Text that uses a font DesignCraft cannot find is highlighted in pink. This is the missing-font highlight. Put the font files in a `Document Fonts` folder beside the file, or install the fonts on your system, so DesignCraft can find them.

## Import warnings

If an IDML package is missing some of its parts, DesignCraft still opens what it can and shows an alert titled **Import Warnings** that names the missing parts. Your other open documents stay as they are. Click **OK** to close the alert.

## Save it again

**File**, then **Save** (Ctrl+S) writes a `.designcraft` file. To hand the layout back to InDesign, use **File**, then **Export IDML…**.
