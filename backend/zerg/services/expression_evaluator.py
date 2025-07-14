"""
Safe Expression Evaluator for Workflow Conditions

Provides secure evaluation of mathematical and logical expressions using simpleeval.
Replaces the string-based conditional evaluation with type-safe AST evaluation.
"""

import logging
from typing import Any
from typing import Dict

from simpleeval import InvalidExpression
from simpleeval import NameNotDefined
from simpleeval import SimpleEval

logger = logging.getLogger(__name__)


class ExpressionEvaluationError(Exception):
    """Raised when expression evaluation fails."""

    pass


class ExpressionValidationError(Exception):
    """Raised when expression validation fails."""

    pass


class ExpressionSecurityError(Exception):
    """Raised when expression violates security restrictions."""

    pass


class SafeExpressionEvaluator:
    """
    Safe expression evaluator using simpleeval.

    Supports:
    - Arithmetic: +, -, *, /, %, **
    - Comparisons: ==, !=, <, <=, >, >=
    - Boolean logic: and, or, not
    - Literals: numbers, strings, booleans, None
    - Parentheses for grouping

    Security:
    - No attribute access (. operator blocked)
    - No function calls except whitelisted built-ins
    - No imports or module access
    - DoS protection (power limits, string limits)
    """

    def __init__(self):
        """Initialize evaluator with security restrictions."""
        self._evaluator = SimpleEval()
        self._configure_security()

    def _configure_security(self):
        """Configure security restrictions for safe evaluation."""

        # Allowed names (built-ins and literals)
        self._evaluator.names = {
            # Boolean literals
            "True": True,
            "False": False,
            "None": None,
        }

        # Safe built-in functions (carefully selected)
        self._evaluator.functions = {
            "abs": abs,
            "min": min,
            "max": max,
            "len": len,
            "str": str,
            "int": int,
            "float": float,
            "bool": bool,
        }

        # Create power limit by overriding the pow operator
        import ast

        original_pow = self._evaluator.operators[ast.Pow]

        def safe_pow(left, right):
            """Safe power operation with limits."""
            if isinstance(right, (int, float)) and right > 100:
                raise ExpressionEvaluationError("Power operation too large (max exponent: 100)")
            return original_pow(left, right)

        self._evaluator.operators[ast.Pow] = safe_pow

    def evaluate(self, expression: str, variables: Dict[str, Any] = None) -> Any:
        """
        Safely evaluate an expression with given variables.

        Args:
            expression: String expression to evaluate (e.g., "a >= 80")
            variables: Dictionary of variable name -> value mappings

        Returns:
            Evaluation result (bool, int, float, str, etc.)

        Raises:
            ExpressionEvaluationError: For evaluation failures
            ExpressionSecurityError: For security violations
            ExpressionValidationError: For invalid expressions
        """
        if not expression or not isinstance(expression, str):
            raise ExpressionValidationError("Expression must be a non-empty string")

        if len(expression) > 500:
            raise ExpressionValidationError("Expression too long (max 500 characters)")

        # Set up variables for evaluation
        if variables:
            # Create a copy to avoid modifying original
            eval_names = {**self._evaluator.names, **variables}
        else:
            eval_names = self._evaluator.names

        try:
            # Set the names for this evaluation
            original_names = self._evaluator.names
            self._evaluator.names = eval_names

            # Use simpleeval for safe evaluation
            result = self._evaluator.eval(expression)

            # Restore original names
            self._evaluator.names = original_names

            logger.debug(f"Expression '{expression}' evaluated to {result} (type: {type(result).__name__})")
            return result

        except InvalidExpression as e:
            raise ExpressionValidationError(f"Invalid expression '{expression}': {e}")

        except NameNotDefined as e:
            raise ExpressionEvaluationError(f"Undefined variable in expression '{expression}': {e}")

        except ZeroDivisionError as e:
            raise ExpressionEvaluationError(f"Division by zero in expression '{expression}': {e}")

        except (OverflowError, ValueError) as e:
            raise ExpressionEvaluationError(f"Mathematical error in expression '{expression}': {e}")

        except Exception as e:
            # Catch any other security violations or unexpected errors
            if "not allowed" in str(e).lower() or "forbidden" in str(e).lower():
                raise ExpressionSecurityError(f"Security violation in expression '{expression}': {e}")
            else:
                raise ExpressionEvaluationError(f"Failed to evaluate expression '{expression}': {e}")

    def validate_expression(self, expression: str) -> bool:
        """
        Validate an expression without evaluating it.

        Args:
            expression: String expression to validate

        Returns:
            True if expression is valid and safe

        Raises:
            ExpressionValidationError: For invalid or unsafe expressions
            ExpressionSecurityError: For security violations
        """
        if not expression or not isinstance(expression, str):
            raise ExpressionValidationError("Expression must be a non-empty string")

        if len(expression) > 500:
            raise ExpressionValidationError("Expression too long (max 500 characters)")

        try:
            # Try to parse without evaluation (dry run with dummy variables)
            dummy_vars = {"a": 1, "b": 2, "c": "test", "d": True}
            original_names = self._evaluator.names
            self._evaluator.names = {**self._evaluator.names, **dummy_vars}

            self._evaluator.eval(expression)

            # Restore original names
            self._evaluator.names = original_names
            return True

        except InvalidExpression as e:
            raise ExpressionValidationError(f"Invalid expression syntax: {e}")

        except Exception as e:
            if "not allowed" in str(e).lower() or "forbidden" in str(e).lower():
                raise ExpressionSecurityError(f"Expression contains forbidden operations: {e}")
            else:
                # For validation, we don't care about undefined variables or math errors
                # Only syntax and security violations matter
                return True

    def get_variable_names(self, expression: str) -> set:
        """
        Extract variable names from an expression.

        Args:
            expression: String expression to analyze

        Returns:
            Set of variable names used in the expression

        Note:
            This is a simple regex-based extraction for basic analysis.
            For complex expressions, actual parsing would be more accurate.
        """
        import re

        # Simple regex to find potential variable names
        # This won't catch all cases but handles common patterns
        pattern = r"\b[a-zA-Z_][a-zA-Z0-9_]*\b"
        potential_vars = set(re.findall(pattern, expression))

        # Remove built-in names and operators
        builtins = set(self._evaluator.names.keys())
        operators = {"and", "or", "not", "in", "is", "True", "False", "None"}

        variables = potential_vars - builtins - operators

        return variables


# Global instance for convenience
safe_evaluator = SafeExpressionEvaluator()


def evaluate_expression(expression: str, variables: Dict[str, Any] = None) -> Any:
    """
    Convenience function for safe expression evaluation.

    Args:
        expression: String expression to evaluate
        variables: Dictionary of variable name -> value mappings

    Returns:
        Evaluation result

    Raises:
        ExpressionEvaluationError: For evaluation failures
        ExpressionSecurityError: For security violations
        ExpressionValidationError: For invalid expressions
    """
    return safe_evaluator.evaluate(expression, variables)


def validate_expression(expression: str) -> bool:
    """
    Convenience function for expression validation.

    Args:
        expression: String expression to validate

    Returns:
        True if expression is valid and safe

    Raises:
        ExpressionValidationError: For invalid expressions
        ExpressionSecurityError: For security violations
    """
    return safe_evaluator.validate_expression(expression)
