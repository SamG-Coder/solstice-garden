#!/usr/bin/env python3
"""Publishes the baked garden, if it changed.

The garden takes minutes to bake and hundreds of megabytes to store, so it is baked on a PC
rather than by CI, and kept in a release of this repository rather than in its history.
scene.txt names that release and the scene's SHA-256; the Pages workflow copies the scene from
the release into the site, since a browser cannot stream a file from a release itself.

This script compares assets/garden.solstice with scene.txt. If the scene changed, it commits a
scene.txt naming a new release, tags that commit, uploads the scene (and the heights its page
walks on) as the release, and only then pushes the branch, which deploys the site: the branch
never names a release that is not there yet. Run again after a failed upload, it carries on.

    python scripts/publish-garden.py [--check] [--bake]

--check only reports what would be published. --bake bakes the garden first.
Needs the GitHub CLI, signed in with permission to create releases and push to this repository.
"""
import datetime
import hashlib
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCENE = "garden.solstice"
# Published beside the scene: the ground's heights and the trunks, for walking.
WALK = "garden.walk"
BAKE = ["cargo", "run", "--release", "-p", "bake", "--", "--garden",
        "--output", f"assets/{SCENE}"]
BRANCH = "main"
HEADER = (
    "# The scene this site serves: its file name, the release of this repository that holds it\n"
    "# (a tag) and its SHA-256. Written by scripts/publish-garden.py; pushing a change deploys\n"
    "# the new scene.\n"
)


def run(command, **kwargs):
    print("$", " ".join(command), flush=True)
    return subprocess.run(command, cwd=ROOT, check=True, **kwargs)


def output(command, check=True):
    result = subprocess.run(command, cwd=ROOT, check=check, capture_output=True, text=True)
    return result.stdout.strip() if result.returncode == 0 else None


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def recorded():
    """What scene.txt names, as (file, release, SHA-256), or None if there is none yet."""
    try:
        with open(os.path.join(ROOT, "scene.txt"), encoding="utf-8") as f:
            for line in f:
                words = line.split("#")[0].split()
                if len(words) == 3:
                    return tuple(words)
    except FileNotFoundError:
        pass
    return None


def new_tag():
    """A tag for today's release, numbered if today already has one."""
    day = datetime.date.today().strftime("garden-%Y-%m-%d")
    taken = set((output(["git", "tag", "--list", f"{day}*"]) or "").split())
    tag, count = day, 1
    while tag in taken:
        count += 1
        tag = f"{day}-{count}"
    return tag


def main():
    args = sys.argv[1:]
    if any(a not in ("--check", "--bake") for a in args):
        sys.exit(__doc__)
    check_only = "--check" in args
    if "--bake" in args:
        run(BAKE)

    scene = os.path.join(ROOT, "assets", SCENE)
    walk = os.path.join(ROOT, "assets", WALK)
    for path in (scene, walk):
        if not os.path.exists(path):
            sys.exit(f"{os.path.relpath(path, ROOT)} is not baked: run with --bake")
    digest = sha256(scene)
    was = recorded()
    if was and was[2] == digest:
        tag = was[1]
        print(f"{SCENE}: unchanged, in {tag}")
    else:
        print(f"{SCENE}: changed")
        if check_only:
            print("would publish it")
            return
        tag = new_tag()
        with open(os.path.join(ROOT, "scene.txt"), "w", encoding="utf-8", newline="\n") as f:
            f.write(HEADER + f"{SCENE} {tag} {digest}\n")
        run(["git", "add", "scene.txt"])
        run(["git", "commit", "-m", f"Serve the garden from {tag}", "--", "scene.txt"])
        run(["git", "tag", tag])

    # What is left to do, which is nothing once a publish has finished.
    held = output(["gh", "release", "view", tag, "--json", "assets"], check=False)
    held = {a["name"]: a["size"] for a in json.loads(held)["assets"]} if held else None
    missing = [p for p in (scene, walk)
               if held is None or held.get(os.path.basename(p)) != os.path.getsize(p)]
    remote = (output(["git", "ls-remote", "origin", f"refs/heads/{BRANCH}"]) or "").split()
    pushed = bool(remote) and remote[0] == output(["git", "rev-parse", BRANCH])
    if check_only or (not missing and pushed):
        if missing:
            print(f"release {tag} still lacks {', '.join(os.path.basename(p) for p in missing)}")
        if not pushed:
            print(f"{BRANCH} is not pushed")
        if not missing and pushed:
            print("nothing to publish")
        return

    if missing:
        run(["git", "push", "origin", f"refs/tags/{tag}"])
        if held is None:
            commit = output(["git", "rev-parse", "--short", f"{tag}^{{commit}}"])
            notes = (f"The garden's scene, baked from {commit}, and the heights its page walks "
                     f"on.\n\n{SCENE}: {os.path.getsize(scene) / 1e6:.1f} MB, "
                     f"SHA-256 `{digest}`.")
            run(["gh", "release", "create", tag, "--verify-tag", "--title", f"Garden {tag[7:]}",
                 "--notes", notes])
        run(["gh", "release", "upload", tag, "--clobber", *missing])
    run(["git", "push", "origin", BRANCH])
    print(f"published as {tag}; the site is deploying")


if __name__ == "__main__":
    main()
