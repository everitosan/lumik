# /// script
# requires-python = ">=3.11,<3.13"
# dependencies = [
#   "torch==2.5.1",
#   "transformers==4.46.3",
#   "sentence-transformers==3.3.1",
#   "onnx==1.17.0",
#   "onnxruntime==1.20.1",
#   "pillow==11.0.0",
#   "numpy<2.2",
# ]
# [tool.uv]
# index-url = "https://download.pytorch.org/whl/cpu"
# extra-index-url = ["https://pypi.org/simple"]
# index-strategy = "unsafe-best-match"
# ///
"""Arma el paquete del modelo de búsqueda por contenido (spec `semantic-search`).

Exporta a ONNX el codificador de imágenes de CLIP ViT-B/32 y el codificador de
texto multilingüe de sentence-transformers, los cuantiza a int8 y empaqueta
`lumik-model-clip-multilingual-v1.zip` con el tokenizer, un `manifest.json` y
las licencias. Imprime el tamaño y el sha256 que la app trae escritos.

Uso (solo para quien publica el modelo):

    uv run tools/models/build_clip_multilingual.py --out <carpeta>

Con `--check <imagen> ...` compara además cada imagen contra unas frases en
español e inglés, con los modelos float y los cuantizados.
"""

import argparse
import hashlib
import json
import shutil
import zipfile
from pathlib import Path

import numpy as np
import torch
from PIL import Image

MODEL_ID = "clip-multilingual-v1"
IMAGE_HF = "openai/clip-vit-base-patch32"
TEXT_HF = "sentence-transformers/clip-ViT-B-32-multilingual-v1"
# Revisiones fijas: el paquete tiene que poder rehacerse igual.
IMAGE_REV = "3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268"
TEXT_REV = "58edf8cada9e398793dca955574a48cbb7f18be2"
IMAGE_SIZE = 224
MEAN = [0.48145466, 0.4578275, 0.40821073]
STD = [0.26862954, 0.26130258, 0.27577711]
MAX_TOKENS = 77

LICENSE_URLS = {
    "LICENSE-clip.txt": "https://raw.githubusercontent.com/openai/CLIP/main/LICENSE",
    "LICENSE-sentence-transformers.txt": "https://raw.githubusercontent.com/UKPLab/sentence-transformers/master/LICENSE",
}


class ImageEncoder(torch.nn.Module):
    def __init__(self, model):
        super().__init__()
        self.model = model

    def forward(self, pixel_values):
        return self.model(pixel_values=pixel_values).image_embeds


class TextEncoder(torch.nn.Module):
    """DistilBERT multilingüe + media de los tokens + capa densa a 512."""

    def __init__(self, st_model):
        super().__init__()
        self.transformer = st_model[0].auto_model
        self.dense = st_model[2].linear

    def forward(self, input_ids, attention_mask):
        hidden = self.transformer(input_ids=input_ids, attention_mask=attention_mask).last_hidden_state
        mask = attention_mask.unsqueeze(-1).to(hidden.dtype)
        pooled = (hidden * mask).sum(1) / mask.sum(1).clamp(min=1e-9)
        return self.dense(pooled)


def resolve_rev(repo: str, wanted: str) -> str:
    """La revisión fija si existe; si no, la actual (y lo avisa para fijarla)."""
    from huggingface_hub import HfApi

    try:
        HfApi().model_info(repo, revision=wanted)
        return wanted
    except Exception:
        sha = HfApi().model_info(repo).sha
        print(f"AVISO: {repo}@{wanted} no existe; se usa {sha}. Fíjala en el script.")
        return sha


def preprocess(path: Path) -> np.ndarray:
    """Lado corto a 224 (bicúbico), recorte central y normalización de CLIP."""
    img = Image.open(path).convert("RGB")
    w, h = img.size
    scale = IMAGE_SIZE / min(w, h)
    img = img.resize((max(IMAGE_SIZE, round(w * scale)), max(IMAGE_SIZE, round(h * scale))), Image.BICUBIC)
    w, h = img.size
    left, top = (w - IMAGE_SIZE) // 2, (h - IMAGE_SIZE) // 2
    img = img.crop((left, top, left + IMAGE_SIZE, top + IMAGE_SIZE))
    arr = (np.asarray(img, dtype=np.float32) / 255.0 - MEAN) / STD
    return arr.transpose(2, 0, 1)[None].astype(np.float32)


def export(out: Path) -> tuple[Path, Path]:
    from sentence_transformers import SentenceTransformer
    from transformers import CLIPVisionModelWithProjection

    out.mkdir(parents=True, exist_ok=True)
    image_rev = resolve_rev(IMAGE_HF, IMAGE_REV)
    text_rev = resolve_rev(TEXT_HF, TEXT_REV)

    vision = CLIPVisionModelWithProjection.from_pretrained(IMAGE_HF, revision=image_rev).eval()
    torch.onnx.export(
        ImageEncoder(vision),
        (torch.zeros(1, 3, IMAGE_SIZE, IMAGE_SIZE),),
        out / "image.f32.onnx",
        input_names=["pixel_values"],
        output_names=["embedding"],
        dynamic_axes={"pixel_values": {0: "batch"}, "embedding": {0: "batch"}},
        opset_version=17,
    )

    st = SentenceTransformer(TEXT_HF, revision=text_rev, device="cpu").eval()
    ids = torch.ones(1, 8, dtype=torch.long)
    torch.onnx.export(
        TextEncoder(st),
        (ids, torch.ones_like(ids)),
        out / "text.f32.onnx",
        input_names=["input_ids", "attention_mask"],
        output_names=["embedding"],
        dynamic_axes={
            "input_ids": {0: "batch", 1: "tokens"},
            "attention_mask": {0: "batch", 1: "tokens"},
            "embedding": {0: "batch"},
        },
        opset_version=17,
    )
    st.tokenizer.backend_tokenizer.save(str(out / "tokenizer.json"))
    (out / "revisions.json").write_text(json.dumps({IMAGE_HF: image_rev, TEXT_HF: text_rev}, indent=2))
    return out / "image.f32.onnx", out / "text.f32.onnx"


def quantize(src: Path, dst: Path) -> None:
    from onnxruntime.quantization import QuantType, quantize_dynamic

    # Todo menos la convolución: ONNX Runtime no tiene ConvInteger en CPU y la
    # convolución del parche es una parte mínima del peso.
    quantize_dynamic(str(src), str(dst), weight_type=QuantType.QInt8, op_types_to_quantize=["MatMul", "Gemm", "Gather"])


def package(out: Path) -> Path:
    import urllib.request

    pkg = out / "pkg"
    if pkg.exists():
        shutil.rmtree(pkg)
    pkg.mkdir()
    shutil.copy(out / "image.onnx", pkg / "image.onnx")
    shutil.copy(out / "text.onnx", pkg / "text.onnx")
    shutil.copy(out / "tokenizer.json", pkg / "tokenizer.json")
    for name, url in LICENSE_URLS.items():
        with urllib.request.urlopen(url) as r:
            (pkg / name).write_bytes(r.read())
    revisions = json.loads((out / "revisions.json").read_text())
    manifest = {
        "id": MODEL_ID,
        "dim": 512,
        "image": {"file": "image.onnx", "size": IMAGE_SIZE, "mean": MEAN, "std": STD, "resize": "shortest_bicubic_center_crop"},
        "text": {"file": "text.onnx", "tokenizer": "tokenizer.json", "max_tokens": MAX_TOKENS},
        "sources": revisions,
    }
    (pkg / "manifest.json").write_text(json.dumps(manifest, indent=2))
    zip_path = out / f"lumik-model-{MODEL_ID}.zip"
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_STORED) as z:
        for f in sorted(pkg.iterdir()):
            z.write(f, f.name)
    return zip_path


def check(out: Path, images: list[Path]) -> None:
    import onnxruntime as ort
    from tokenizers import Tokenizer

    phrases = [
        "dos gatos en un sillón", "two cats on a couch",
        "un dormitorio con una cama", "a bedroom with a bed",
        "niños con un trofeo de tenis", "kids with a tennis trophy",
        "un partido de béisbol", "a baseball game",
        "coches en una calle", "cars on a street",
        "una novia en la playa", "a bride on the beach",
    ]
    tok = Tokenizer.from_file(str(out / "tokenizer.json"))
    tok.enable_truncation(MAX_TOKENS)
    tok.enable_padding()
    enc = tok.encode_batch(phrases)
    ids = np.array([e.ids for e in enc], dtype=np.int64)
    mask = np.array([e.attention_mask for e in enc], dtype=np.int64)
    pixels = np.concatenate([preprocess(p) for p in images])

    def norm(v):
        return v / np.linalg.norm(v, axis=1, keepdims=True)

    for kind in ["f32.", ""]:
        img = ort.InferenceSession(str(out / f"image.{kind}onnx"))
        txt = ort.InferenceSession(str(out / f"text.{kind}onnx"))
        iv = norm(img.run(None, {"pixel_values": pixels})[0])
        tv = norm(txt.run(None, {"input_ids": ids, "attention_mask": mask})[0])
        sims = iv @ tv.T
        print(f"\n== {'float32' if kind else 'int8'}")
        for p, row in zip(images, sims):
            best = sorted(zip(row, phrases), reverse=True)[:4]
            print(f"{p.name}: " + " | ".join(f"{s:.3f} {t}" for s, t in best))
        if kind:
            ref = (iv, tv)
        else:
            print(f"parecido int8 vs float32: imagen {np.mean(np.sum(iv * ref[0], 1)):.4f}, texto {np.mean(np.sum(tv * ref[1], 1)):.4f}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--check", type=Path, nargs="*", default=[])
    ap.add_argument("--skip-build", action="store_true", help="solo --check, con lo ya generado en --out")
    args = ap.parse_args()
    if args.skip_build:
        check(args.out, args.check)
        return

    image_f32, text_f32 = export(args.out)
    quantize(image_f32, args.out / "image.onnx")
    quantize(text_f32, args.out / "text.onnx")
    zip_path = package(args.out)
    digest = hashlib.sha256(zip_path.read_bytes()).hexdigest()
    print(f"\n{zip_path.name}: {zip_path.stat().st_size} bytes, sha256 {digest}")
    for f in ["image.f32.onnx", "text.f32.onnx", "image.onnx", "text.onnx"]:
        print(f"  {f}: {(args.out / f).stat().st_size / 1e6:.1f} MB")
    if args.check:
        check(args.out, args.check)


if __name__ == "__main__":
    main()
