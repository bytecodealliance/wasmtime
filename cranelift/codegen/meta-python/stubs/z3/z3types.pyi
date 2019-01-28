from typing import Any

class Z3Exception(Exception):
  def __init__(self, a: Any) -> None:
    self.value = a
    ...

class ContextObj:
  ...

class Ast:
  ...
