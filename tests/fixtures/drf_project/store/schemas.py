from pydantic import BaseModel


class OrderSchema(BaseModel):
    email: str
    total: float

    class Config:
        frozen = True

    def summary_line(self) -> str:
        return f"{self.email}: {self.total}"
