from django.db.models.signals import post_save
from django.dispatch import receiver

from .models import Product


@receiver(post_save, sender=Product)
def handle_product_saved(sender, instance, **kwargs):
    instance.display_price()
