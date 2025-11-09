curl -X POST -u "root:root" -H "Surreal-NS: dev" -H "Surreal-DB: rtwalk" -H "Accept: application/json" -d "DEFINE ANALYZER english_analyzer TOKENIZERS blank FILTERS lowercase, snowball(english);" http://localhost:4003/sql

curl -X POST -u "root:root" -H "Surreal-NS: dev" -H "Surreal-DB: rtwalk" -H "Accept: application/json" -d "DEFINE INDEX user_username_index ON user FIELDS username SEARCH ANALYZER english_analyzer BM25;" http://localhost:4003/sql

curl -X POST -u "root:root" -H "Surreal-NS: dev" -H "Surreal-DB: rtwalk" -H "Accept: application/json" -d "DEFINE INDEX user_display_name_index ON user FIELDS display_name SEARCH ANALYZER english_analyzer BM25;" http://localhost:4003/sql

curl -X POST -u "root:root" -H "Surreal-NS: dev" -H "Surreal-DB: rtwalk" -H "Accept: application/json" -d "DEFINE INDEX user_bio_index ON user FIELDS bio SEARCH ANALYZER english_analyzer BM25;" http://localhost:4003/sql

curl -X POST -u "root:root" -H "Surreal-NS: dev" -H "Surreal-DB: rtwalk" -H "Accept: application/json" -d "DEFINE INDEX forum_name_index ON forum FIELDS name SEARCH ANALYZER english_analyzer BM25;" http://localhost:4003/sql

curl -X POST -u "root:root" -H "Surreal-NS: dev" -H "Surreal-DB: rtwalk" -H "Accept: application/json" -d "DEFINE INDEX forum_display_name_index ON forum FIELDS display_name SEARCH ANALYZER english_analyzer BM25;" http://localhost:4003/sql

curl -X POST -u "root:root" -H "Surreal-NS: dev" -H "Surreal-DB: rtwalk" -H "Accept: application/json" -d "DEFINE INDEX forum_description_index ON forum FIELDS description SEARCH ANALYZER english_analyzer BM25;" http://localhost:4003/sql


curl -X POST -u "root:root" -H "Surreal-NS: dev" -H "Surreal-DB: rtwalk" -H "Accept: application/json" -d "DEFINE INDEX post_title_index ON post FIELDS title SEARCH ANALYZER english_analyzer BM25;" http://localhost:4003/sql

curl -X POST -u "root:root" -H "Surreal-NS: dev" -H "Surreal-DB: rtwalk" -H "Accept: application/json" -d "DEFINE INDEX post_content_index ON post FIELDS content SEARCH ANALYZER english_analyzer BM25;" http://localhost:4003/sql
