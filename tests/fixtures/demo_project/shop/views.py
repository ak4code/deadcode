from .models import Product


def product_list(request):
    return list(Product.objects.all())


def abandoned_view(request):
    return Product.objects.none()
