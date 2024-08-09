## EASY MUSIC LIBRARY

A music library server, including server, cli client, and webui. Supports muitiple artists per track and album.

### Screen Shots




https://github.com/yuygfgg/easy_musiclib/assets/140488233/9d9ad2b4-c3d5-4b4a-9faa-0ac6fe2a8cd8

<img width="1268" alt="截屏2024-08-09 22 02 18" src="https://github.com/user-attachments/assets/060fa715-df3c-40fd-8bee-f97b79d05a62">
<img width="1266" alt="截屏2024-08-09 22 02 53" src="https://github.com/user-attachments/assets/ae94fddf-ff60-46b6-88fe-45dd7397eff8">
<img width="1267" alt="截屏2024-08-09 22 03 05" src="https://github.com/user-attachments/assets/52b080e7-3fe2-40ed-9827-689bc93c8546">
<img width="1272" alt="截屏2024-08-09 22 03 55" src="https://github.com/user-attachments/assets/7b386ee1-3e2e-4bb3-ae81-9b096aa73d2e">
<img width="1269" alt="截屏2024-08-09 22 04 48" src="https://github.com/user-attachments/assets/edbc9f2e-bd6a-440d-8f22-2b3bd75ef7a3">
<img width="1268" alt="截屏2024-08-09 22 04 57" src="https://github.com/user-attachments/assets/2289b93e-414f-4afa-a0e3-ccfb0d3ab2fe">
<img width="1272" alt="截屏2024-08-09 22 05 27" src="https://github.com/user-attachments/assets/2924f478-ffc4-46da-8994-eb09ab19a3ce">



### Schedual

- mobile app : ~ 2025

The project will continue unless I find another music library server that handles muitiple artist and muiti-disc albums correctly.

Webui is currently in **Chinese** only.

### Usage

#### Server Setup
1. Clone the repository
2. Install all necessary python packages ``` pip3 install -r requirements.txt ```
3. Run server ```python3 server.py ```

#### Scan Directory
1. Goto settings page
2. Enter your music folder and click Scan button

#### Merge Unintendedly Splitted Artists
The programme splits artists with delimiters, so single artist with these delimiters in name will be splitted unintendedly.
1. Goto settings page.
2. Add a new artist with the single artist's name (e.g. A&B/C)
3. Perform merge artist by name (or by uuid if you want) multiple times. (e.g. 1. merge A&B/C with A 2. merge A&B/C with B 3. merge A&B/C with C)

#### Webui
1. Webui is avaliable at http://server_address:5010/

#### API
1. APIs are at http://server_address:5010/api/


### Contribution

Any kind of contribution is welcomed, especiall for GUI apps, which I know nothing about.
