from src.models.user import User

def get_user(user_id):
    return User(user_id, "Test User")