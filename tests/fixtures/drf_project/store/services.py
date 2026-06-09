from .schemas import OrderSchema


def compute_discount(order_total):
    return order_total / 10


def apply_discount(order):
    discounted_total = compute_discount(order.total)
    return OrderSchema(email=order.email, total=discounted_total)


def dead_service():
    return None
