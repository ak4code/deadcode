from rest_framework import viewsets

from .models import Order
from .permissions import IsOrderOwner
from .serializers import OrderSerializer
from .services import apply_discount


class OrderViewSet(viewsets.ModelViewSet):
    queryset = Order.objects.all()
    serializer_class = OrderSerializer
    permission_classes = [IsOrderOwner]

    def perform_create(self, serializer):
        apply_discount(serializer.instance)
