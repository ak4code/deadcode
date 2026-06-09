def compute_discount(order_total):
    return order_total / 10


def apply_discount(order):
    return compute_discount(order.total)


def dead_service():
    return None
