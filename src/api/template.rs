use crate::{
    client::Client,
    error::{Error, Result},
    models::{Template, TemplateCreateRequest, TemplateBuild},
};
use reqwest::StatusCode;

#[derive(Clone)]
pub struct TemplateApi {
    client: Client,
}

impl TemplateApi {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn list(&self) -> Result<Vec<Template>> {
        let url = self.client.build_url("/templates");
        let response = self.client.http().get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let templates: Vec<Template> = response.json().await?;
                Ok(templates)
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn get(&self, template_id: &str) -> Result<Template> {
        let url = self.client.build_url(&format!("/templates/{}", template_id));
        let response = self.client.http().get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let template: Template = response.json().await?;
                Ok(template)
            }
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Template {}", template_id))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn create(&self, request: TemplateCreateRequest) -> Result<TemplateInstance> {
        let url = self.client.build_url("/templates");
        let response = self
            .client
            .http()
            .post(&url)
            .json(&request)
            .send()
            .await?;

        match response.status() {
            StatusCode::CREATED | StatusCode::OK => {
                let template: Template = response.json().await?;
                Ok(TemplateInstance {
                    api: self.clone(),
                    template,
                })
            }
            StatusCode::UNAUTHORIZED => Err(Error::Authentication("Invalid API key".to_string())),
            StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimit),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub fn name(self, name: impl Into<String>) -> TemplateBuilder {
        TemplateBuilder::new(self.client, name.into())
    }
}

pub struct TemplateBuilder {
    client: Client,
    request: TemplateCreateRequest,
}

impl TemplateBuilder {
    fn new(client: Client, name: String) -> Self {
        Self {
            client,
            request: TemplateCreateRequest {
                name,
                description: None,
                dockerfile: String::new(),
                start_cmd: None,
                cpu_count: None,
                memory_mb: None,
                disk_mb: None,
            },
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.request.description = Some(desc.into());
        self
    }

    pub fn dockerfile(mut self, dockerfile: impl Into<String>) -> Self {
        self.request.dockerfile = dockerfile.into();
        self
    }

    pub fn start_cmd(mut self, cmd: impl Into<String>) -> Self {
        self.request.start_cmd = Some(cmd.into());
        self
    }

    pub fn cpu_count(mut self, count: u32) -> Self {
        self.request.cpu_count = Some(count);
        self
    }

    pub fn memory_mb(mut self, memory: u32) -> Self {
        self.request.memory_mb = Some(memory);
        self
    }

    pub fn disk_mb(mut self, disk: u32) -> Self {
        self.request.disk_mb = Some(disk);
        self
    }

    pub async fn create(self) -> Result<TemplateInstance> {
        let api = TemplateApi::new(self.client);
        api.create(self.request).await
    }
}

pub struct TemplateInstance {
    api: TemplateApi,
    template: Template,
}

impl TemplateInstance {
    pub fn id(&self) -> &str {
        &self.template.template_id
    }

    pub fn template(&self) -> &Template {
        &self.template
    }

    pub async fn rebuild(&self) -> Result<TemplateBuild> {
        let url = self.api.client.build_url(&format!("/templates/{}/builds", self.template.template_id));
        let response = self.api.client.http().post(&url).send().await?;

        match response.status() {
            StatusCode::CREATED | StatusCode::OK => {
                let build: TemplateBuild = response.json().await?;
                Ok(build)
            }
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Template {}", self.template.template_id))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn builds(&self) -> Result<Vec<TemplateBuild>> {
        let url = self.api.client.build_url(&format!("/templates/{}/builds", self.template.template_id));
        let response = self.api.client.http().get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let builds: Vec<TemplateBuild> = response.json().await?;
                Ok(builds)
            }
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Template {}", self.template.template_id))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn delete(self) -> Result<()> {
        let url = self.api.client.build_url(&format!("/templates/{}", self.template.template_id));
        let response = self.api.client.http().delete(&url).send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(Error::NotFound(format!("Template {}", self.template.template_id))),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(Error::Api {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.template = self.api.get(&self.template.template_id).await?;
        Ok(())
    }
}