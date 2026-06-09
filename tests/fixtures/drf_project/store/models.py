from django.db import models


class BaseTimestamped(models.Model):
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        abstract = True


class Order(BaseTimestamped):
    email = models.EmailField()
    total = models.DecimalField(max_digits=10, decimal_places=2)
