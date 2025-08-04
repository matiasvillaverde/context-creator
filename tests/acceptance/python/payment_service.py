#!/usr/bin/env python3
# Copyright The OpenTelemetry Authors
# SPDX-License-Identifier: Apache-2.0

import time
import random
from decimal import Decimal
from typing import Dict, Optional

from opentelemetry import trace
from opentelemetry.trace import Status, StatusCode

# Get tracer
tracer = trace.get_tracer("payment-service")


class PaymentService:
    """Service for processing payments"""
    
    def __init__(self):
        self.payment_gateway_url = "https://payment-gateway.example.com"
    
    def process_payment(self, user_id: str, amount: Decimal, card_number: str) -> Dict:
        """Process a payment transaction"""
        with tracer.start_as_current_span("process_payment") as span:
            span.set_attributes({
                "user.id": user_id,
                "payment.amount": float(amount),
                "payment.currency": "USD"
            })
            
            # Validate card
            if not self.validate_card(card_number):
                span.set_status(Status(StatusCode.ERROR, "Invalid card"))
                raise ValueError("Invalid card number")
            
            # Check fraud
            fraud_score = self.check_fraud(user_id, amount)
            span.set_attribute("fraud.score", fraud_score)
            
            if fraud_score > 0.8:
                span.set_status(Status(StatusCode.ERROR, "Fraud detected"))
                raise Exception("Payment declined due to fraud detection")
            
            # Process with gateway
            transaction_id = self.charge_card(card_number, amount)
            span.set_attribute("transaction.id", transaction_id)
            
            return {
                "transaction_id": transaction_id,
                "status": "success",
                "amount": float(amount)
            }
    
    def validate_card(self, card_number: str) -> bool:
        """Validate credit card number"""
        with tracer.start_as_current_span("validate_card") as span:
            # Simulate validation delay
            time.sleep(0.002)
            
            is_valid = len(card_number) == 16 and card_number.isdigit()
            span.set_attribute("card.valid", is_valid)
            
            return is_valid
    
    def check_fraud(self, user_id: str, amount: Decimal) -> float:
        """Check for fraudulent transaction"""
        with tracer.start_as_current_span("check_fraud") as span:
            span.set_attributes({
                "user.id": user_id,
                "check.amount": float(amount)
            })
            
            # Simulate fraud check
            time.sleep(0.01)
            score = random.random()
            
            # Higher amounts have higher fraud probability
            if amount > 1000:
                score = min(score * 1.5, 1.0)
            
            span.set_attribute("fraud.score", score)
            return score
    
    def charge_card(self, card_number: str, amount: Decimal) -> str:
        """Charge the credit card"""
        with tracer.start_as_current_span("charge_card") as span:
            span.set_attribute("payment.gateway", "stripe")
            
            # Simulate API call to payment gateway
            time.sleep(0.05)
            
            transaction_id = f"txn_{int(time.time())}_{random.randint(1000, 9999)}"
            span.set_attribute("transaction.id", transaction_id)
            
            return transaction_id
    
    def refund_payment(self, transaction_id: str, amount: Optional[Decimal] = None) -> Dict:
        """Process a refund for a transaction"""
        with tracer.start_as_current_span("refund_payment") as span:
            span.set_attributes({
                "transaction.id": transaction_id,
                "refund.partial": amount is not None
            })
            
            if amount:
                span.set_attribute("refund.amount", float(amount))
            
            # Simulate refund processing
            time.sleep(0.03)
            
            refund_id = f"ref_{int(time.time())}"
            span.set_attribute("refund.id", refund_id)
            
            return {
                "refund_id": refund_id,
                "transaction_id": transaction_id,
                "status": "completed"
            }