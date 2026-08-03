"""Chainlit UI entry point (MVP placeholder).

A real conversational UI will be built here in later steps. For now the
module exists to reserve the integration point and document the contract.
"""

import chainlit as cl


@cl.on_chat_start
async def on_chat_start() -> None:
    await cl.Message(content="Welcome to Nereus! The AI tutor will be available soon.").send()
