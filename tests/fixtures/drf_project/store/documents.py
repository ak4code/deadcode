from django_elasticsearch_dsl import Document
from django_elasticsearch_dsl.registries import registry

from .models import Order


class BaseSearchIndex(Document):
    def serialize_payload(self, instance):
        return {"id": instance.pk}


@registry.register_document
class OrderIndex(BaseSearchIndex):
    class Index:
        name = "orders"

    class Django:
        model = Order

    def prepare_email(self, instance):
        return instance.email.lower()

    def serialize_payload(self, instance):
        payload = super().serialize_payload(instance)
        payload["email"] = instance.email
        return payload
