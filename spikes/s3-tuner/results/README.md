# S3 tuner results

Raw output of the S3 spike (ADR-029). Throwaway evidence; conclusions go in ADR-029 §5.

- `synthetic.txt` — A9 accuracy on synthetic sine and plucked tones (clean and 20 dB SNR),
  plus the chromatic sine sweep E2..E6, for MPM and YIN. Reproduce: `cargo run --release -- synthetic`.
- `latency.txt` — A8: hops to the first stable reading per open string, MPM and YIN.
  Reproduce: `cargo run --release -- latency`.

## Real-guitar set (REQUIRED before ADR-029 leaves Draft, §4)

The coding agent cannot record a physical instrument. To produce the real-guitar numbers,
record on the developer's guitar + interface and drop 16-bit PCM mono/stereo WAVs here,
named by the open string they contain:

```
E2_soft.wav  E2_hard.wav  E2_capo2.wav
A2_soft.wav  A2_hard.wav
D3_mf.wav    G3_mf.wav    B3_mf.wav    E4_mf.wav
```

Each take: pluck the open string, let it ring ~2 s, several dynamics (soft to hard), with
and without a capo for at least E2 and A2. 48 kHz preferred (other rates are measured but
noted). Then:

```
cargo run --release -- wav results
```

This prints per-file mean/p95 cents, octave-error and detection rates for MPM and YIN.
Record the instrument, interface, room and sample count in ADR-029 §5, and copy the table
into it. Do not invent these numbers.

## Live mic check (A10)

`cargo run --release -- input 10` opens the default input device, prints the typed outcome
(Ok / NoInputDevice / PermissionDenied / DeviceBusyOrConfig) and live readings while you
pluck. For the permission and busy cases, toggle the Windows microphone privacy setting or
hold the device in another app, as in the MVP manual script.
