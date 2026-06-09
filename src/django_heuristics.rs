use std::path::Path;

/// Последние сегменты декораторов, помечающих функцию как точку входа.
///
/// Сюда входят сигналы Django, задачи Celery, шаблонные теги
/// и фикстуры Pytest.
const ENTRY_POINT_DECORATOR_SEGMENTS: &[&str] =
    &["receiver", "shared_task", "task", "fixture", "simple_tag"];

/// Атрибуты классов `admin.ModelAdmin`, содержащие строковые ссылки на код.
pub const ADMIN_DYNAMIC_ATTRIBUTES: &[&str] =
    &["list_display", "list_filter", "actions", "readonly_fields"];

/// Встроенные функции, принимающие имя атрибута строкой.
pub const DYNAMIC_REFERENCE_BUILTINS: &[&str] = &["getattr", "setattr", "hasattr", "delattr"];

/// Функции регистрации маршрутов Django.
pub const URL_REGISTRATION_FUNCTIONS: &[&str] = &["path", "re_path", "url"];

/// Имена методов, вызываемых фреймворком Django неявно.
///
/// Такие методы переопределяют поведение базовых классов и не имеют
/// явных вызовов в коде проекта.
const IMPLICIT_METHOD_NAMES: &[&str] = &[
    "handle",
    "add_arguments",
    "ready",
    "save",
    "delete",
    "clean",
    "full_clean",
    "get_queryset",
    "get_object",
    "get_context_data",
    "get_absolute_url",
    "get_success_url",
    "form_valid",
    "form_invalid",
    "setUp",
    "tearDown",
    "setUpTestData",
    "has_add_permission",
    "has_change_permission",
    "has_delete_permission",
    "has_view_permission",
];

/// Имена классов, обнаруживаемых Django по соглашению.
const IMPLICIT_CLASS_NAMES: &[&str] = &["Meta", "Media", "DoesNotExist", "MultipleObjectsReturned"];

/// Имена переменных модулей, читаемых Django по соглашению.
const IMPLICIT_VARIABLE_NAMES: &[&str] = &[
    "urlpatterns",
    "app_name",
    "application",
    "default_app_config",
    "handler400",
    "handler403",
    "handler404",
    "handler500",
];

/// Нормализует текст декоратора до точечного имени без аргументов.
///
/// :param decorator_text: Полный текст узла декоратора, включая символ `@`.
/// :return: Точечное имя декоратора без скобок и аргументов.
pub fn normalize_decorator_expression(decorator_text: &str) -> String {
    let without_at_sign = decorator_text.trim_start_matches('@').trim();
    let without_arguments = without_at_sign.split('(').next().unwrap_or(without_at_sign);
    without_arguments.trim().to_string()
}

/// Проверяет принадлежность декоратора к точкам входа.
///
/// :param normalized_decorator: Нормализованное точечное имя декоратора.
/// :return: Признак точки входа.
pub fn is_entry_point_decorator(normalized_decorator: &str) -> bool {
    let last_segment = last_dotted_segment(normalized_decorator);
    if ENTRY_POINT_DECORATOR_SEGMENTS.contains(&last_segment) {
        return true;
    }
    normalized_decorator.ends_with("register.filter")
}

/// Проверяет регистрацию класса через декоратор `admin.register`.
///
/// :param normalized_decorator: Нормализованное точечное имя декоратора.
/// :return: Признак регистрации класса в админке.
pub fn is_admin_register_decorator(normalized_decorator: &str) -> bool {
    last_dotted_segment(normalized_decorator) == "register"
}

/// Проверяет принадлежность файла к директории management команд Django.
///
/// :param file_path: Путь к анализируемому файлу.
/// :return: Признак файла management команды.
pub fn is_management_command_path(file_path: &Path) -> bool {
    let path_components: Vec<&str> = file_path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect();
    path_components
        .windows(2)
        .any(|window| window == ["management", "commands"])
}

/// Проверяет неявное использование метода фреймворком Django.
///
/// :param method_name: Простое имя метода.
/// :return: Признак неявного вызова метода.
pub fn is_implicit_method_name(method_name: &str) -> bool {
    IMPLICIT_METHOD_NAMES.contains(&method_name)
}

/// Проверяет обнаружение класса фреймворком Django по соглашению.
///
/// :param class_name: Простое имя класса.
/// :return: Признак неявного использования класса.
pub fn is_implicit_class_name(class_name: &str) -> bool {
    IMPLICIT_CLASS_NAMES.contains(&class_name)
}

/// Проверяет чтение переменной модуля фреймворком Django по соглашению.
///
/// :param variable_name: Простое имя переменной.
/// :return: Признак неявного использования переменной.
pub fn is_implicit_variable_name(variable_name: &str) -> bool {
    IMPLICIT_VARIABLE_NAMES.contains(&variable_name)
}

/// Проверяет принадлежность модуля к настройкам Django.
///
/// Все переменные модулей настроек считаются используемыми, поскольку
/// Django читает их через `django.conf.settings`.
///
/// :param module_path: Точечный путь модуля.
/// :return: Признак модуля настроек.
pub fn is_settings_module(module_path: &str) -> bool {
    module_path.split('.').any(|segment| segment == "settings")
}

/// Проверяет принадлежность класса к конфигурации приложения Django.
///
/// :param module_path: Точечный путь модуля.
/// :param superclasses_text: Текст списка базовых классов.
/// :return: Признак класса `AppConfig` в модуле `apps`.
pub fn is_app_config_class(module_path: &str, superclasses_text: &str) -> bool {
    let last_module_segment = last_dotted_segment(module_path);
    last_module_segment == "apps" && superclasses_text.contains("AppConfig")
}

/// Проверяет имя на соответствие протоколу dunder.
///
/// :param entity_name: Простое имя сущности.
/// :return: Признак специального имени Python.
pub fn is_dunder_name(entity_name: &str) -> bool {
    entity_name.len() > 4 && entity_name.starts_with("__") && entity_name.ends_with("__")
}

/// Возвращает последний сегмент точечного имени.
///
/// :param dotted_name: Точечное имя вида `module.attribute`.
/// :return: Последний сегмент имени.
pub fn last_dotted_segment(dotted_name: &str) -> &str {
    dotted_name.rsplit('.').next().unwrap_or(dotted_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorator_normalization_strips_arguments() {
        assert_eq!(
            normalize_decorator_expression("@receiver(post_save, sender=Product)"),
            "receiver"
        );
        assert_eq!(normalize_decorator_expression("@app.task"), "app.task");
    }

    #[test]
    fn entry_point_decorators_are_recognized() {
        assert!(is_entry_point_decorator("receiver"));
        assert!(is_entry_point_decorator("app.task"));
        assert!(is_entry_point_decorator("shared_task"));
        assert!(is_entry_point_decorator("pytest.fixture"));
        assert!(is_entry_point_decorator("register.simple_tag"));
        assert!(is_entry_point_decorator("register.filter"));
        assert!(!is_entry_point_decorator("property"));
        assert!(!is_entry_point_decorator("staticmethod"));
    }

    #[test]
    fn management_command_paths_are_recognized() {
        assert!(is_management_command_path(Path::new(
            "shop/management/commands/sync_products.py"
        )));
        assert!(!is_management_command_path(Path::new("shop/views.py")));
    }
}
