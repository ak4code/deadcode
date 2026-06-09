from rest_framework import serializers

from .models import Order


def normalize_email(value):
    return value.strip().lower()


class BaseOrderSerializer(serializers.ModelSerializer):
    def validate(self, attrs):
        return attrs


class OrderSerializer(BaseOrderSerializer):
    total_display = serializers.SerializerMethodField()

    class Meta:
        model = Order
        fields = "__all__"

    def validate_email(self, value):
        return normalize_email(value)

    def get_total_display(self, obj):
        return str(obj.total)

    def create(self, validated_data):
        return Order.objects.create(**validated_data)
