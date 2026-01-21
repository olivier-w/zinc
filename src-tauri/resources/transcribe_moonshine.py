#!/usr/bin/env python3
"""
Moonshine transcription script using sherpa-onnx Python API.
Called by Zinc app for transcription.

Handles long audio files by chunking into segments to avoid context length issues.
Moonshine models were tested on clips up to ~55 seconds, so we use conservative chunks.
"""

import sys
import json
import argparse

# Maximum chunk duration in seconds (Moonshine works best with shorter clips)
MAX_CHUNK_SECONDS = 30

def main():
    parser = argparse.ArgumentParser(description='Transcribe audio using Moonshine')
    parser.add_argument('audio_file', help='Path to audio file (WAV format, 16kHz)')
    parser.add_argument('--preprocessor', required=True, help='Path to preprocess.onnx')
    parser.add_argument('--encoder', required=True, help='Path to encoder model')
    parser.add_argument('--uncached-decoder', required=True, help='Path to uncached decoder')
    parser.add_argument('--cached-decoder', required=True, help='Path to cached decoder')
    parser.add_argument('--tokens', required=True, help='Path to tokens file')
    parser.add_argument('--num-threads', type=int, default=4, help='Number of threads')
    args = parser.parse_args()

    import sherpa_onnx
    import wave
    import struct

    # Create recognizer using Moonshine factory method
    try:
        recognizer = sherpa_onnx.OfflineRecognizer.from_moonshine(
            preprocessor=args.preprocessor,
            encoder=args.encoder,
            uncached_decoder=args.uncached_decoder,
            cached_decoder=args.cached_decoder,
            tokens=args.tokens,
            num_threads=args.num_threads,
            debug=False,
        )
    except Exception as e:
        sys.stderr.write(f"Failed to create Moonshine recognizer: {e}\n")
        raise

    # Load audio
    with wave.open(args.audio_file, 'rb') as f:
        sample_rate = f.getframerate()
        num_frames = f.getnframes()
        raw_samples = f.readframes(num_frames)
        sample_width = f.getsampwidth()

        # Convert to float
        if sample_width == 2:
            samples = struct.unpack(f'{num_frames}h', raw_samples)
            samples = [s / 32768.0 for s in samples]
        elif sample_width == 4:
            samples = struct.unpack(f'{num_frames}i', raw_samples)
            samples = [s / 2147483648.0 for s in samples]
        else:
            raise ValueError(f"Unsupported sample width: {sample_width}")

    # Calculate chunk size in samples
    chunk_samples = int(MAX_CHUNK_SECONDS * sample_rate)
    total_samples = len(samples)

    # Process in chunks if audio is longer than MAX_CHUNK_SECONDS
    all_text = []
    all_timestamps = []
    all_tokens = []
    time_offset = 0.0

    num_chunks = (total_samples + chunk_samples - 1) // chunk_samples
    sys.stderr.write(f"Processing {total_samples / sample_rate:.1f}s audio in {num_chunks} chunks of {MAX_CHUNK_SECONDS}s\n")

    for chunk_idx, chunk_start in enumerate(range(0, total_samples, chunk_samples)):
        chunk_end = min(chunk_start + chunk_samples, total_samples)
        chunk = samples[chunk_start:chunk_end]

        sys.stderr.write(f"Processing chunk {chunk_idx + 1}/{num_chunks} ({len(chunk) / sample_rate:.1f}s)\n")

        # Create stream and decode this chunk
        stream = recognizer.create_stream()
        stream.accept_waveform(sample_rate, chunk)
        recognizer.decode_stream(stream)

        # Collect results
        chunk_text = stream.result.text
        chunk_timestamps = list(stream.result.timestamps) if hasattr(stream.result, 'timestamps') else []
        chunk_tokens = list(stream.result.tokens) if hasattr(stream.result, 'tokens') else []

        if chunk_text:
            all_text.append(chunk_text.strip())
            sys.stderr.write(f"  Got text: {chunk_text[:50]}...\n" if len(chunk_text) > 50 else f"  Got text: {chunk_text}\n")

        # Adjust timestamps by adding the time offset for this chunk
        for ts in chunk_timestamps:
            all_timestamps.append(ts + time_offset)

        all_tokens.extend(chunk_tokens)

        # Update time offset for next chunk
        time_offset += len(chunk) / sample_rate

    # Output JSON result
    result = {
        "text": " ".join(all_text),
        "timestamps": all_timestamps,
        "tokens": all_tokens,
    }

    print(json.dumps(result))

if __name__ == '__main__':
    main()
