import importlib


def dynamic_target():
    return "ok"


def resolve_callable(module_name: str):
    module = importlib.import_module(module_name)
    return getattr(module, "dynamic_target")


LEGACY_FLAG = True
