from rest_framework.permissions import BasePermission


class IsOrderOwner(BasePermission):
    def has_permission(self, request, view):
        return request.user.is_authenticated

    def has_object_permission(self, request, view, obj):
        return obj.email == request.user.email
