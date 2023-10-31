#!/usr/bin/env python

import os

# must be set before importing cv2
os.environ["OPENCV_IO_ENABLE_OPENEXR"] = "1"

import cv2
import subprocess
import glob


def convert_hdr(path_in, path_out):
    img = cv2.imread(path_in, cv2.IMREAD_ANYCOLOR | cv2.IMREAD_ANYDEPTH)

    tonemap = cv2.createTonemapReinhard(2.2, 0.0, 0.0, 0.0)
    img = tonemap.process(img)
    cv2.imwrite(path_out, img * 255)


# Build the project
subprocess.run(["cargo", "build", "--release", "--features", "enable_oidn"])

# Set TEV_PATH to an empty string
os.environ["TEV_PATH"] = ""

# Find and process .toml files in the "examples" directory
for toml_path in glob.iglob("examples/**/*.toml", recursive=True):
    print(f"Rendering {toml_path}")

    # Run the target/release/pbrtrs_main command
    subprocess.run(["target/release/pbrtrs_main", toml_path])

    print(f"Converting {toml_path}")

    # Run the "magick" command to convert out.exr to .png
    convert_hdr("out.exr", f"{os.path.splitext(toml_path)[0]}.png")
