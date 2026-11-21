# Russian dictation model comparison, 2026-11-08

> One-off comparison behind the switch to large-v3 (change `openspec/changes/archive/2026-11-08-switch-dictation-to-large-v3/`, v0.47.0). The script stayed outside the repository; the models are in `~/.voicepuncher/models/research/stt-bench/`.

Input: 100 FLEURS `ru_ru` test clips and 45 of the user's dictations (15 with English terms), whisper models run with the app's `whisper-server` flags.

| Model | FLEURS ru WER | RTF (Metal) | On the user's speech |
|---|---|---|---|
| large-v3 f16, beam 5 | 5.8 % | 0.16 | fewest meaning errors (about 6 of 45, by reading) |
| large-v3 f16, greedy | — | 0.14 | misses several hard clips beam 5 fixes |
| large-v3 q5_0 | 5.9 % | 0.18 | no gain over f16 |
| large-v3-turbo q5_0 | 6.2 % | 0.08 | about 10 of 45 with a meaning error |
| large-v3-turbo q8_0 | 6.3 % | — | no gain over q5_0 |
| GigaAM v3 (punctuated) | 7.1 % | 0.03 | mixes Latin letters into Cyrillic words on English terms ("кlodcod диsайn") |
| Parakeet v3 | 8.5 % | — | garbles English terms |

All whisper variants fall within 5.8–6.3 % on FLEURS; the user's own speech (fast, unclear Russian with English terms) is what tells them apart.

- **large-v3 f16 with beam 5 won**, at twice turbo's recognition time. Its flaw: about 1 clip in 6 comes out lowercase without punctuation (without a prompt 9 of 45 lacked punctuation, turbo 8).
- **GigaAM v3** is the fastest but unusable for mixed Russian–English speech without post-processing.
- **Next model change:** start from large-v3 f16 with beam 5; try a prompt against the lowercase output first.
