#!/usr/bin/env python3
"""
Parakeet transcription script using sherpa-onnx with CUDA support.
Called by Zinc app for GPU-accelerated transcription.

Handles long audio files by chunking into segments to avoid memory/context limits.
"""

import os
import sys
import json
import ctypes
import argparse

# Maximum chunk duration in seconds (5 minutes is safe for most GPUs)
MAX_CHUNK_SECONDS = 300

def preload_cuda_dlls():
    """Preload CUDA DLLs in correct order for Windows."""
    if sys.platform != 'win32':
        return True

    # Find sherpa_onnx lib path
    try:
        import sherpa_onnx
        lib_path = os.path.join(os.path.dirname(sherpa_onnx.__file__), 'lib')
    except ImportError:
        return False

    if not os.path.exists(lib_path):
        return False

    os.add_dll_directory(lib_path)

    cuda_dlls = [
        'cudart64_12.dll',
        'cublas64_12.dll',
        'cublasLt64_12.dll',
        'cudnn64_9.dll',
        'cudnn_ops64_9.dll',
        'cudnn_cnn64_9.dll',
        'cudnn_adv64_9.dll',
        'cudnn_graph64_9.dll',
        'cudnn_heuristic64_9.dll',
        'cudnn_engines_precompiled64_9.dll',
        'cudnn_engines_runtime_compiled64_9.dll',
    ]

    loaded_count = 0
    for dll in cuda_dlls:
        dll_path = os.path.join(lib_path, dll)
        if os.path.exists(dll_path):
            try:
                ctypes.CDLL(dll_path)
                loaded_count += 1
            except Exception:
                pass

    # Return True if we loaded at least the core CUDA DLLs
    return loaded_count >= 3

def main():
    parser = argparse.ArgumentParser(description='Transcribe audio using Parakeet TDT')
    parser.add_argument('audio_file', help='Path to audio file (WAV format, 16kHz)')
    parser.add_argument('--encoder', required=True, help='Path to encoder model')
    parser.add_argument('--decoder', required=True, help='Path to decoder model')
    parser.add_argument('--joiner', required=True, help='Path to joiner model')
    parser.add_argument('--tokens', required=True, help='Path to tokens file')
    parser.add_argument('--provider', default='cuda', choices=['cuda', 'cpu'], help='Execution provider')
    parser.add_argument('--num-threads', type=int, default=4, help='Number of threads')
    args = parser.parse_args()

    # Preload CUDA DLLs if using CUDA
    cuda_available = False
    if args.provider == 'cuda':
        cuda_available = preload_cuda_dlls()
        if not cuda_available:
            sys.stderr.write("CUDA DLLs not found, falling back to CPU\n")
            args.provider = 'cpu'

    import sherpa_onnx
    import wave
    import struct

    # Create recognizer
    try:
        recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
            encoder=args.encoder,
            decoder=args.decoder,
            joiner=args.joiner,
            tokens=args.tokens,
            model_type='nemo_transducer',
            provider=args.provider,
            num_threads=args.num_threads,
        )
    except Exception as e:
        # Fall back to CPU if CUDA fails
        if args.provider == 'cuda':
            sys.stderr.write(f"CUDA failed, falling back to CPU: {e}\n")
            recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
                encoder=args.encoder,
                decoder=args.decoder,
                joiner=args.joiner,
                tokens=args.tokens,
                model_type='nemo_transducer',
                provider='cpu',
                num_threads=args.num_threads,
            )
        else:
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

    for chunk_start in range(0, total_samples, chunk_samples):
        chunk_end = min(chunk_start + chunk_samples, total_samples)
        chunk = samples[chunk_start:chunk_end]

        # Create stream and decode this chunk
        stream = recognizer.create_stream()
        stream.accept_waveform(sample_rate, chunk)
        recognizer.decode_stream(stream)

        # Collect results
        chunk_text = stream.result.text
        chunk_timestamps = list(stream.result.timestamps)
        chunk_tokens = list(stream.result.tokens)

        if chunk_text:
            all_text.append(chunk_text)

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
