let api_base_url = "/api/v0/";

if (import.meta.env.MODE !== "production") {
	api_base_url = "http://127.0.0.1:8000/" + api_base_url;
}

console.log(api_base_url);
