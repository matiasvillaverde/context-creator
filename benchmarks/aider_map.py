"""Offline adapter for aider-chat==0.86.0's actual RepoMap implementation.

Bypasses the interactive CLI and substitutes local BPE accounting for a model
client. Ranking, AST tags, rendering and cache behavior are Aider's own code.
"""
import random
import sys
from pathlib import Path
from types import SimpleNamespace
import tiktoken
from aider.repomap import RepoMap

random.seed(0)
root = Path(sys.argv[1]).resolve()
encoder = tiktoken.get_encoding("o200k_base")
io = SimpleNamespace(
    read_text=lambda path: Path(path).read_text(encoding="utf-8"),
    tool_output=lambda *args, **kwargs: None,
    tool_warning=lambda *args, **kwargs: print(*args, file=sys.stderr),
    tool_error=lambda *args, **kwargs: print(*args, file=sys.stderr),
)
model = SimpleNamespace(token_count=lambda text: len(encoder.encode_ordinary(text)))
mapper = RepoMap(map_tokens=int(sys.argv[2]), root=str(root), main_model=model,
                 io=io, map_mul_no_files=1, refresh="always")
files = sorted(str(path) for path in root.rglob("*") if path.is_file()
               and not any(part.startswith(".") for part in path.relative_to(root).parts))
print(mapper.get_repo_map([], files) or "", end="")
