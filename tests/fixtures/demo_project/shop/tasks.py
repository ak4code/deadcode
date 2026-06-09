from celery import shared_task


@shared_task
def refresh_catalog():
    return _collect_catalog_rows()


def _collect_catalog_rows():
    return []


def forgotten_helper():
    return None
