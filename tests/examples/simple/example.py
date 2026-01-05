"""Simple example module for testing."""


def greet(name: str) -> str:
    """Return a greeting message.

    >>> greet("World")
    'Hello, World!'
    >>> greet("Alice")
    'Hello, Alice!'
    """
    return f"Hello, {name}!"


if __name__ == "__main__":
    print(greet("World"))
