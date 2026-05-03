# Alert Sounds

Drop a short (~0.5s) MP3/WAV here named `alert-critical.mp3` and update
`alerts.html` `<audio id="alerts-chime">` `src` from the inline data-URI
placeholder to `/static/audio/alert-critical.mp3`.

Recommended properties:
- ≤ 1 second duration
- 24 dB headroom (volume slider in topbar attenuates 0..100%)
- Distinct from system OS sounds (avoid bell, ding)
- WAV or MP3, mono, 44.1 kHz

Free CC0 sources:
- https://freesound.org (filter: CC0)
- https://mixkit.co/free-sound-effects/notification/
