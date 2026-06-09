from django.core.management.base import BaseCommand

from shop.models import Product


class Command(BaseCommand):
    def handle(self, *args, **options):
        Product.objects.update(price=0)
