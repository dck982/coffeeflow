#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "google-genai",
#     "python-dotenv",
# ]
# ///
"""Generate a WAV file from text using the Gemini TTS API.

Usage:
    uv run generate_tts.py "Espresso shot started" -o startup.wav
"""

import argparse
import os
import wave
from pathlib import Path

from dotenv import load_dotenv
from google import genai
from google.genai import types

MODEL = "gemini-2.5-flash-preview-tts"
VOICE = "Kore"
SAMPLE_RATE = 24000


def generate(text: str, output_path: Path, voice: str = VOICE) -> None:
    client = genai.Client(api_key=os.environ["GOOGLE_API_KEY"])

    response = client.models.generate_content(
        model=MODEL,
        contents=text,
        config=types.GenerateContentConfig(
            response_modalities=["AUDIO"],
            speech_config=types.SpeechConfig(
                voice_config=types.VoiceConfig(
                    prebuilt_voice_config=types.PrebuiltVoiceConfig(voice_name=voice)
                )
            ),
        ),
    )

    pcm_data = response.candidates[0].content.parts[0].inline_data.data

    with wave.open(str(output_path), "wb") as wav_file:
        wav_file.setnchannels(1)
        wav_file.setsampwidth(2)  # 16-bit PCM
        wav_file.setframerate(SAMPLE_RATE)
        wav_file.writeframes(pcm_data)

    print(f"Wrote {output_path} ({len(pcm_data)} bytes PCM)")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("text", help="Text to synthesize")
    parser.add_argument("-o", "--output", type=Path, default=Path("output.wav"))
    parser.add_argument("--voice", default=VOICE, help=f"Voice name (default: {VOICE})")
    args = parser.parse_args()

    load_dotenv()
    generate(args.text, args.output, args.voice)


if __name__ == "__main__":
    main()
