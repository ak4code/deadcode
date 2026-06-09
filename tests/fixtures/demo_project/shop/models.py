from functools import cached_property

from django.db import models


class Product(models.Model):
    title = models.CharField(max_length=255)
    price = models.DecimalField(max_digits=10, decimal_places=2)

    @property
    def availability_label(self) -> str:
        """Используется только в шаблоне product_detail.html."""
        return "В наличии"

    @cached_property
    def slug(self) -> str:
        """Используется только в шаблоне product_list.html."""
        return self.title.lower().replace(" ", "-")

    def display_price(self) -> str:
        """
        Возвращает цену товара для отображения в админке.

        :return: Строка с ценой и валютой.
        """
        return f"{self.price} RUB"

    def unused_helper(self) -> str:
        return self.title.upper()
