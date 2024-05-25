from starlette.requests import Request
import ray
from ray import serve
import onceuponai
import lancedb
import requests
import os

JWKS = requests.get("https://login.botframework.com/v1/.well-known/keys").text

PROMPT_TEMPLATE = """[INST]Skorzystaj z poniższych fragmentów kontekstu, aby odpowiedzieć na pytanie na końcu. 
Jeśli nie znasz odpowiedzi, po prostu powiedz, że nie wiesz, nie próbuj wymyślać odpowiedzi. 

Kontekst: 
{context}

Pytanie:
{question}[/INST]"""


@serve.deployment(num_replicas=1, ray_actor_options={"num_cpus": 1, "num_gpus": 1})
class RecipesChat:
    async def __init__(self):

        self.llm_model = onceuponai.llms.Quantized("speakleash/Bielik-7B-Instruct-v0.1-GGUF", \
                              "file:///home/jovyan/rust-src/Bielik/Bielik-7B-Instruct-v0.1.Q4_K_S.gguf", \
                              "speakleash/Bielik-7B-Instruct-v0.1", "cuda")
        
        self.embeddings = onceuponai.embeddings.E5("intfloat/multilingual-e5-small")

        db = await lancedb.connect_async("az://recipesv2/vectors2")
        self.lancedb_table = await db.open_table("recipes_vectors")

    async def chat(self, prompt: str) -> str:
        emb = self.embeddings.embed([prompt])
        res = await self.lancedb_table.query().nearest_to(emb[0]).limit(1).to_pandas()
        print("EMB",res)
        context = res.to_dict()["item"][0]
        prompt = PROMPT_TEMPLATE.replace("{context}", context).replace("{question}", prompt)
        print("PROMPT", prompt)
        return await self.llm_model.invoke(prompt, 2000)
    
    def auth_token(self):
        bot_client_id = os.environ["BOT_CLIENT_ID"]
        bot_client_secret = os.environ["BOT_CLIENT_SECRET"]
        headers = {"Content-Type": "application/x-www-form-urlencoded"}
        body = f"grant_type=client_credentials&client_id={bot_client_id}&client_secret={bot_client_secret}&scope=https%3A%2F%2Fapi.botframework.com%2F.default"
        resp = requests.post("https://login.microsoftonline.com/botframework.com/oauth2/v2.0/token", headers=headers, data=body).json()["access_token"]

        return resp
    
    def reply_bot(self, auth_token, incommind_message, text):
        reply_message = {
            "type": "message",
            "from": incommind_message["recipient"],
            "conversation": incommind_message["conversation"],
            "recipient": incommind_message["from"],
            "text": text,
            "replyToId": incommind_message["id"]
        }

        headers ={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {auth_token}"
        }
        url = f"{incommind_message['serviceUrl']}v3/conversations/{incommind_message['conversation']['id']}/activities/{incommind_message['id']}"
        requests.post(url, headers=headers, json=reply_message)

    async def __call__(self, http_request: Request) -> str:
        request_text: str = await http_request.json()
        jwt = http_request.headers['authorization'].replace("Bearer ", "")
        is_valid = await onceuponai.bot.validate_jwt(jwt, JWKS)
        if is_valid:
            print(request_text)
            text = await self.chat(request_text['text'])
            print(text)

            auth_state = self.auth_token()

            self.reply_bot(auth_state, request_text, text)

serve.start(http_options={"host": "0.0.0.0", "port": 8080})
recipes_app = RecipesChat.bind()
