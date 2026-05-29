  JUnit 5 + JaCoCo (SoTA for Java unit testing).

  Gradle (build.gradle.kts):
    plugins { id("jacoco") }
    dependencies {
        testImplementation(platform("org.junit:junit-bom:5.11.4"))
        testImplementation("org.junit.jupiter:junit-jupiter")
        testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    }
    tasks.test { useJUnitPlatform(); finalizedBy(tasks.jacocoTestReport) }
    tasks.jacocoTestReport {
        dependsOn(tasks.test)
        reports { xml.required = true; html.required = true; csv.required = false }
    }
    tasks.jacocoTestCoverageVerification {
        violationRules {
            rule { limit { minimum = "0.{{THRESHOLD}}".toBigDecimal() } }
        }
    }

  Maven (pom.xml):
    <plugin>
      <groupId>org.jacoco</groupId>
      <artifactId>jacoco-maven-plugin</artifactId>
      <version>0.8.12</version>
      <executions>
        <execution><goals><goal>prepare-agent</goal></goals></execution>
        <execution>
          <id>report</id><phase>test</phase><goals><goal>report</goal></goals>
        </execution>
        <execution>
          <id>check</id><goals><goal>check</goal></goals>
          <configuration><rules><rule>
            <limits><limit>
              <counter>LINE</counter><value>COVEREDRATIO</value><minimum>0.{{THRESHOLD}}</minimum>
            </limit></limits>
          </rule></rules></configuration>
        </execution>
      </executions>
    </plugin>

  Run via skill:
    bash <skill>/scripts/run.sh --stacks java
