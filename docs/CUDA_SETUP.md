# CUDA Setup For RTX 4060 Ti

## Current State

- GPU detected: `NVIDIA GeForce RTX 4060 Ti`
- PyTorch in this repo is already CUDA-enabled: `torch 2.11.0+cu130`
- NVIDIA kernel modules are loaded
- NVIDIA driver is installed: `580.126.18`
- Current blocker: `/dev/nvidia*` device nodes are missing, so `nvidia-smi` and PyTorch cannot access the GPU

## Fix

Run:

```bash
sudo apt-get update
sudo apt-get install -y nvidia-modprobe
sudo nvidia-modprobe -u -c=0
```

## Verify

Check the driver:

```bash
nvidia-smi
```

Check PyTorch CUDA:

```bash
python3 - <<'PY'
import torch
print(torch.cuda.is_available())
if torch.cuda.is_available():
    print(torch.cuda.get_device_name(0))
    print(torch.cuda.get_device_capability(0))
    print(torch.version.cuda)
PY
```

## Expected Result

- `nvidia-smi` should show the RTX 4060 Ti
- `torch.cuda.is_available()` should print `True`
- the repo should then be able to run CUDA-backed kernel tests and training

## Notes

- This repo does not currently need a different PyTorch build
- The issue is system-level NVIDIA device access, not the project virtualenv
