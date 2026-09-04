# Tom/Tomy orb states

Place the final orb artwork in this folder. Use square transparent PNG files at **512 x 512 px**; this remains crisp for the 176 px orb on high-DPI displays without bloating the binary.

Required filenames:

- `idle.png` — calm default state
- `listening.png` — receiving user input
- `thinking.png` — reasoning or searching memory
- `working.png` — executing an approved action
- `success.png` — task completed
- `curious.png` — a new pattern or suggestion
- `warning.png` — attention required, non-destructive
- `error.png` — an action failed
- `sleeping.png` — paused or background mode

Keep the silhouette, lighting direction, size, and internal texture consistent across every state. Change color, glow, deformation, and particle intensity to communicate emotion. Do not place text in the images.

These assets are embedded at build time and used in the dashboard, onboarding, and compact floating-orb mode.
