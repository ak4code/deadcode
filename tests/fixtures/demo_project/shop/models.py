from django.db import models


class Product(models.Model):
    title = models.CharField(max_length=255)
    price = models.DecimalField(max_digits=10, decimal_places=2)

    def display_price(self) -> str:
        """
        Возвращает цену товара для отображения в админке.

        :return: Строка с ценой и валютой.
        """
        return f"{self.price} RUB"

    def unused_helper(self) -> str:
        return self.title.upper()
